use winit::{
    event::{Event,WindowEvent},
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use winit::dpi::PhysicalSize;
use tokio;

use winit_input_helper::WinitInputHelper;

mod buffers;
mod render;
mod texture;
mod egui_manager;
mod ui_panels;
mod sprite_manager;
mod slot_object;
mod input;
mod init;

use egui_manager::EguiManager;
use ui_panels::UiState;


use buffers::*;
use render::*;
use sprite_manager::*;
use slot_object::*;
use input::*;
use init::*;

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

    //sprites

    let mut cursor: Sprite = Sprite::new(&wgpu_app.device, &wgpu_app.queue, 
        "./tex/cursor/def_cursor.png", [0,0], [1,1]);
    cursor.translation = [4.0,4.0,0.0,1.0];
    cursor.build_buffers(&wgpu_app.device);

    let mut map:Vec<Sprite> = Vec::new();
    let decor:Vec<Sprite> = Vec::new();

    for i in 0..10{
        for j in 0..10{
            let mut block: Sprite = Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/floor.png", [0,0], [2,2]);
            block.translation = [i as f32 - 4.0, j as f32 - 4.0, 0.0, 1.0];
            block.build_buffers(&wgpu_app.device);

            map.push(block);
        }
    }

    let mut game : GameObjects = GameObjects {cursor, map, decor, groups: Vec::new()};

    //main loop vars
    let mut input = WinitInputHelper::new();

    let mut mode = 0;

    //slots

    let mut act_slot:i32 = 0;
    let mut slots:Vec<Slot> = vec![
        Slot{
            id: 1,
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/table.png", [1,0], [2,1]),
                width: 2, height: 1, name: String::from("table"),
            },
            active: true,
        },
        Slot{
            id: 2,
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/carpet.png", [0,0], [2,2]),
                width: 1, height: 1, name: String::from("carpet"),
            },
            active: false,
        },
        Slot{
            id: 3,
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/sign.png", [0,0], [1,1]),
                width: 1, height: 1, name: String::from("lamp"),
            },
            active: false,
        },
    ];

    //INIT UV
    let mut egui_manager = EguiManager::new(
        &wgpu_app.device,
        wgpu_app.surface_format,
        None,  // depth format
        1,     // samples
        &window,
    );

    let mut ui_state = UiState::new(
        mode,
        vec![
            Slot {
                id: 1,
                obj: Object {
                    sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, 
                        "tex/decor/table.png", [0,0], [2,1]),
                    width: 2, height: 1, name: String::from("table"),
                },
                active: true,
            },
            Slot {
                id: 2,
                obj: Object {
                    sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/carpet.png", [0,0], [2,2]),
                    width: 1, height: 1, name: String::from("carpet"),
                },
                active: false,
            },
            Slot{
                id: 3,
                obj: Object{
                    sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/sign.png", [0,0], [1,1]),
                    width: 1, height: 1, name: String::from("lamp"),
                },
                active: false,
            },
        ],
    );

    // main loop
    let _ = event_loop.run(|event, event_loop_target| {

        if let Event::WindowEvent { event, window_id } = &event {
            if *window_id == window.id() {
                egui_manager.handle_input(&window, event);
            }
        }

        //Input
        if input.update(&event) {
            let (new_act_slot, new_mode) = do_input(
                &wgpu_app.device, &wgpu_app.queue, &input,
                &mut game, &mut slots, act_slot, mode,
                &mut ui_state,
            );
            act_slot = new_act_slot;
            mode = new_mode;
        }

        let uniforms = Uniforms { translation: [0.0, 0.0, 0.0, 0.0], 
            rotation: [0.0, 0.0, 0.0, 0.0], _padding: [0.0; 3],};
        wgpu_app.queue.write_buffer(&wgpu_app.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        //Render

        match event {
            //Exit
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window_id => {
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

                let mut opaque_models = vec![];
                opaque_models.extend(game.map.iter());

                let mut transparent_models = vec![];
                transparent_models.extend(game.decor.iter());
                transparent_models.push(&game.cursor);

                render(
                    &surface, &wgpu_app.device, &wgpu_app.queue, &wgpu_app.render_pipeline, 
                    &wgpu_app.transparent_pipeline,
                    &wgpu_app.bind_group, &wgpu_app.depth_buffer.view,
                    &mut egui_manager,
                    &window,
                    |ctx| ui_state.render(ctx),
                    &opaque_models, &transparent_models
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