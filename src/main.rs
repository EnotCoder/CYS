use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::PhysicalSize,
};
use std::fs::File;
use std::io::{BufRead, BufReader};

mod api_components;
mod sprite_manager;
mod slot_object;
mod input;
mod ecs;
mod text_renderer;
mod scene;

use api_components::*;
use sprite_manager::*;
use slot_object::Slot;
use ecs::*;

use specs::{WorldExt, Builder};

const WORLD_OFFSET_X: f32 = -11.0;
const WORLD_OFFSET_Y: f32 = 11.0;

const EMPTY_UNIFORMS: Uniforms = Uniforms {
    translation: [0.0; 4],
    rotation: [0.0; 4],
    _padding: [0.0; 3],
};

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("CYS — Create your Shop")
        .with_inner_size(PhysicalSize::new(800, 800))
        .build(&event_loop)
        .unwrap();

    let mut wgpu_app = WgpuApp::new(&window).await;
    let surface = wgpu_app
        .instance
        .create_surface(&window)
        .expect("Failed to create surface");

    let mut text_renderer = text_renderer::TextRenderer::new("font.otf");
    let mut scene_manager = scene::SceneManager::new(&mut text_renderer);

    let mut input = winit_input_helper::WinitInputHelper::new();

    let _ = event_loop.run(|event, event_loop_target| {
        let window_size = (
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );

        if input.update(&event) {
            if let Some(scene) = scene_manager.scenes.get_mut(&scene_manager.current) {
                let action = scene.update(
                    &mut scene_manager.ecs,
                    &input,
                    window_size,
                    &mut text_renderer,
                    &wgpu_app.device,
                    &wgpu_app.queue,
                );
                match action {
                    scene::SceneAction::Switch(name) => {
                        scene_manager.switch_to(&name, &mut text_renderer);
                    }
                    scene::SceneAction::None => {}
                }
            }
        }

        wgpu_app
            .queue
            .write_buffer(&wgpu_app.uniform_buffer, 0, bytemuck::cast_slice(&[EMPTY_UNIFORMS]));

        let ms = scene_manager.scenes.get(&scene_manager.current).unwrap().map_size();
        let size_data = Size { map_size: ms };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));

        let ui_uniforms = UiUniforms {
            size: 1.0,
            _padding: [0.0; 3],
        };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => event_loop_target.exit(),

            Event::AboutToWait => window.request_redraw(),

            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let (map_sprites, carpet_sprites, decor_sprites, cursor_sprites, ui_sprites) =
                    scene_manager.scenes.get(&scene_manager.current).unwrap().sprites(&scene_manager.ecs);

                render(
                    &surface,
                    &wgpu_app.device,
                    &wgpu_app.queue,
                    &wgpu_app.render_pipeline,
                    &wgpu_app.transparent_pipeline,
                    &wgpu_app.depth_buffer.view,
                    &map_sprites,
                    &carpet_sprites,
                    &decor_sprites,
                    &cursor_sprites,
                    &ui_sprites,
                    &wgpu_app.size_bind_group,
                    &wgpu_app.ui_bind_group,
                    &mut scene_manager.ecs.sprite_cache,
                );
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } if new_size.width > 0 && new_size.height > 0 => {
                wgpu_app.config.width = new_size.width;
                wgpu_app.config.height = new_size.height;
                surface.configure(&wgpu_app.device, &wgpu_app.config);
                wgpu_app.depth_buffer.resize(&wgpu_app.device, new_size);
                window.request_redraw();
            }

            _ => {}
        }
    });
}

pub(crate) fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open("map.txt").expect("map.txt not found!");
    let reader = BufReader::new(file);

    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        for (i, token) in parts.iter().enumerate() {
            let (tex_path, tex_pos, tex_count) = match *token {
                "0" | "0.1" | "0.2" | "0.3" => grass_frame(token),
                "1" => ("tex/floor.png", [0, 0], [2, 2]),
                token if ["2","3","4","5","6","7","8","9","8.1","9.1"].contains(&token) => {
                    wall_frame(token)
                }
                _ => ("tex/floor.png", [0, 0], [2, 2]),
            };

            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;

            ecs.world
                .create_entity()
                .with(Transform {
                    position: [x, y, 0.0],
                })
                .with(SpriteComponent {
                    texture_path: tex_path.to_string(),
                    texture_frame: tex_pos,
                    texture_count: tex_count,
                })
                .build();
        }
    }
}

fn grass_frame(code: &str) -> (&str, [i32; 2], [i32; 2]) {
    match code {
        "0" => ("tex/grass.png", [0, 0], [3, 2]),
        "0.1" => ("tex/grass.png", [1, 0], [3, 2]),
        "0.2" => ("tex/grass.png", [0, 1], [3, 2]),
        "0.3" => ("tex/grass.png", [2, 1], [3, 2]),
        _ => ("tex/grass.png", [0, 0], [3, 2]),
    }
}

fn wall_frame(code: &str) -> (&str, [i32; 2], [i32; 2]) {
    let (col, row): (i32, i32) = match code {
        "2" => (0, 0),
        "3" => (0, 1),
        "4" => (1, 0),
        "5" => (1, 1),
        "6" => (0, 2),
        "7" => (1, 2),
        "8" => (0, 3),
        "8.1" => (0, 4),
        "9" => (1, 3),
        "9.1" => (1, 4),
        _ => (0, 0),
    };
    ("tex/wall.png", [col, row], [2, 5])
}

