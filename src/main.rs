use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::PhysicalSize,
};
mod api_components;
mod sprite_manager;
mod slot_object;
mod input;
mod ecs;
mod text_renderer;
mod scene;
mod map_loader;
mod fps;
mod inventory;
mod pathfinding;
mod constants;
mod util;

pub use map_loader::load_map_to_ecs;
use crate::constants::*;

use api_components::*;
use sprite_manager::*;
use slot_object::Slot;
use ecs::*;
use fps::FpsCounter;

const EMPTY_UNIFORMS: Uniforms = Uniforms {
    translation: [0.0; 4],
    rotation: [0.0; 4],
    _padding: [0.0; 3],
};

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let window = WindowBuilder::new()
        .with_title("CYS — Create your Shop")
        .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
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
    let mut fps_counter = FpsCounter::new();

    let _ = event_loop.run(|event, event_loop_target| {
        let window_size = (
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );

        input.update(&event);

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
                let action = {
                    let scene = scene_manager.scenes.get_mut(&scene_manager.current).unwrap();
                    scene.update(
                        &mut scene_manager.ecs,
                        &input,
                        window_size,
                        &mut text_renderer,
                        &wgpu_app.device,
                        &wgpu_app.queue,
                    )
                };
                match action {
                    scene::SceneAction::Switch(name) => {
                        scene_manager.switch_to(&name, &mut text_renderer);
                    }
                    scene::SceneAction::Quit => {
                        event_loop_target.exit();
                    }
                    scene::SceneAction::None => {}
                }

                let fps = fps_counter.tick();
                scene_manager.update_fps(fps, &mut text_renderer, &wgpu_app.device, &wgpu_app.queue);

                wgpu_app
                    .queue
                    .write_buffer(&wgpu_app.uniform_buffer, 0, bytemuck::cast_slice(&[EMPTY_UNIFORMS]));

                let ms = scene_manager.scenes.get(&scene_manager.current).unwrap().map_size();
                let aspect = window_size.0 / window_size.1;
                let size_data = Size { map_size: ms, aspect };
                wgpu_app
                    .queue
                    .write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));

                let ui_uniforms = UiUniforms { size: 1.0, aspect, _padding: [0.0; 2] };
                wgpu_app
                    .queue
                    .write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

                let (map_sprites, carpet_sprites, decor_sprites, npc_sprites, cursor_sprites, ui_sprites) =
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
                    &npc_sprites,
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
