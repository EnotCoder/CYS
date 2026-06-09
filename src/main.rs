use winit::{
    event::{Event,WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::PhysicalSize,
};
use tokio;
use winit_input_helper::WinitInputHelper;

use std::fs::File;
use std::io::{BufRead, BufReader};

mod api_components;
mod sprite_manager;
mod slot_object;
mod input;
mod ecs;

use api_components::*;
use sprite_manager::*;
use slot_object::*;
use input::*;
use components::*;
use ecs::*;

use specs::{WorldExt, Builder};


#[tokio::main]
async fn main() {

    let event_loop = EventLoop::new().unwrap();

    //winit window
    let window = WindowBuilder::new()
        .with_title("game")
        .with_inner_size(PhysicalSize::new(800, 800))
        .build(&event_loop)
        .unwrap();

    let mut wgpu_app = WgpuApp::new(&window).await;


    //поверхность
    let surface = wgpu_app.instance.create_surface(&window)
        .expect("Failed to create surface");

    let mut ecs = EcsAdapter::new();
    
    // Создаем курсор через ECS (z=0.2 для слоя курсора)
    let cursor_entity = ecs.add_cursor(0.0, 0.0, "tex/cursor/def_cursor.png");

    load_map_to_ecs(&mut ecs);
    let (icon_button) = get_uv_ecs(&mut ecs);

    //slots

    let mut act_slot:i32 = 0;
    let mut slots:Vec<Slot> = get_slot_vec();

    //main loop vars
    let mut input = WinitInputHelper::new();
    let mut mode = 0;

    let mut map_size:f32 = 0.8;

    // main loop
    let _ = event_loop.run(|event, event_loop_target| {
        //Input
        if input.update(&event) {
            let (new_act_slot, new_mode, new_size) = do_input(
                &wgpu_app.device, &wgpu_app.queue, &input,
                &mut ecs, &mut slots, act_slot, mode,
                map_size, 
                
                cursor_entity,
                icon_button,
            );
            act_slot = new_act_slot;
            mode = new_mode;
            map_size = new_size;
        }

        let uniforms = Uniforms { translation: [0.0, 0.0, 0.0, 0.0], 
            rotation: [0.0, 0.0, 0.0, 0.0], _padding: [0.0; 3],};
        wgpu_app.queue.write_buffer(&wgpu_app.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let size = Size{map_size};
        wgpu_app.queue.write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size]));

        let ui_uniforms = UiUniforms { size: 1.0, _padding: [0.0; 3]  };  // всегда 1.0 (не меняется)
        wgpu_app.queue.write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

        //Render

        match event {
            //Exit
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                event_loop_target.exit();
            }
            //Redraw window
            Event::AboutToWait => {
                window.request_redraw();
            }
            //Render
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let (map_sprites, carpet_sprites, decor_sprites, cursor_sprites, ui_sprites) = ecs.get_sprites_by_layer();
                
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
                    &wgpu_app.bind_group,
                    &wgpu_app.size_bind_group,
                    &wgpu_app.ui_bind_group,
                    &mut ecs.sprite_cache,
                );

            }

            //Window resize
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                window_id,
            } if window_id == window.id() => {
                // Обновляем существующую конфигурацию
                wgpu_app.config.width = new_size.width;
                wgpu_app.config.height = new_size.height;
                surface.configure(&wgpu_app.device, &wgpu_app.config);
                
                wgpu_app.depth_buffer.resize(&wgpu_app.device, new_size);

                // Запрашиваем перерисовку
                window.request_redraw();
            }

            // Игнорируем все остальные события
            _ => (),
        }
    });

}

const WORLD_OFFSET_X: f32 = -11.0;
const WORLD_OFFSET_Y: f32 = 11.0;

fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open("map.txt").unwrap();
    let reader = BufReader::new(file);

    let mut j = 0;
    
    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        for i in 0..parts.len() {
            let (tex_path, tex_pos, tex_cut) = match parts[i] {
                "0" => ("tex/grass.png", [0, 0], [2, 2]),
                "1" => ("tex/floor.png", [0, 0], [2, 2]),
                "2" => ("tex/wall.png", [0, 0], [2, 5]),
                "3" => ("tex/wall.png", [0, 1], [2, 5]),
                "4" => ("tex/wall.png", [1, 0], [2, 5]),
                "5" => ("tex/wall.png", [1, 1], [2, 5]),
                "6" => ("tex/wall.png", [0, 2], [2, 5]),
                "7" => ("tex/wall.png", [1, 2], [2, 5]),
                "8" => ("tex/wall.png", [0, 3], [2, 5]),
                "8.1" => ("tex/wall.png", [0, 4], [2, 5]),
                "9" => ("tex/wall.png", [1, 3], [2, 5]),
                "9.1" => ("tex/wall.png", [1, 4], [2, 5]),
                _ => ("tex/floor.png", [0, 0], [2, 2]),
            };
            
            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;
            
            ecs.world.create_entity()
                .with(Transform {
                    position: [x, y, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                })
                .with(SpriteComponent {
                    texture_path: tex_path.to_string(),
                    texture_frame: tex_pos,
                    texture_count: tex_cut,
                    z_index: 0,
                })
                .build();
        }
        
        j += 1;
    }
}

fn get_uv_ecs(
    ecs: &mut EcsAdapter
) -> (specs::Entity) {
    let icon_mode = ecs.add_ui(
        4.0, -4.0,
        "tex/ui/mode/standart_mode.png",
    );

    (icon_mode)
}
