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

use api_components::*;
use sprite_manager::*;
use slot_object::*;
use input::*;

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
        "tex/cursor/def_cursor.png", [0,0], [1,1]);
        
    cursor.translation = [0.0,0.0,0.0,1.0];
    cursor.build_buffers(&wgpu_app.device);

    let mut map:Vec<Sprite> = Vec::new();

    let _ = match get_map(&wgpu_app, &mut map){
        Err(e) => eprintln!("Error {e}"),
        _ => (),
    };

    let decor:Vec<Sprite> = Vec::new();
    let carpets:Vec<Sprite> = Vec::new();

    //My UI
    let ui: Ui = Ui::new(&wgpu_app.device, &wgpu_app.queue, get_slot_vec(&wgpu_app));

    //game struct
    let mut game : GameObjects = GameObjects {cursor, map, carpets, decor, groups: Vec::new(), ui};

    //slots

    let mut act_slot:i32 = 0;
    let mut slots:Vec<Slot> = get_slot_vec(&wgpu_app);

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
                &mut game, &mut slots, act_slot, mode,
                map_size,
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

                let mut opaque_models = vec![];
                opaque_models.extend(game.carpets.iter());
                opaque_models.extend(game.map.iter());

                let mut transparent_models = vec![];
                transparent_models.extend(game.decor.iter());
                transparent_models.push(&game.cursor);

                let mut ui_model = vec![];
                ui_model.push(&game.ui.mode_icon);
                ui_model.extend(game.ui.slots_icons.iter());
                ui_model.push(&game.ui.cursor_slot);

                render(
                    &surface, &wgpu_app.device, &wgpu_app.queue,
                    &wgpu_app.render_pipeline,
                    &wgpu_app.transparent_pipeline,
                    &wgpu_app.depth_buffer.view,
                    &opaque_models,
                    &transparent_models,
                    &ui_model,
                    &wgpu_app.bind_group,
                    &wgpu_app.size_bind_group,
                    &wgpu_app.ui_bind_group,
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


fn get_slot_vec(wgpu_app: &WgpuApp) -> Vec<Slot>{
    vec![
        Slot {
            obj: Object {
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/box.png", [0,0], [1,1]),
                width: 1, height: 1, name: String::from("box"),
            },
            active: true,
        },
        Slot {
            obj: Object {
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/carpet.png", [0,0], [2,2]),
                width: 1, height: 1, name: String::from("carpet"),
            },
            active: false,
        },
        Slot{
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/sign.png", [0,0], [1,1]),
                width: 1, height: 1, name: String::from("sing"),
            },
            active: false,
        },
        Slot{
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/table.png", [1,0], [2,1]),
                width: 2, height: 1, name: String::from("table"),
            },
            active: false,
        },
        Slot{
            obj: Object{
                sprite: Sprite::new(&wgpu_app.device, &wgpu_app.queue, "tex/decor/rack.png", [0,0], [1,2]),
                width: 1, height: 2, name: String::from("rack"),
            },
            active: false,
        },
    ]
}

const WORLD_OFFSET_X: f32 = -11.0;
const WORLD_OFFSET_Y: f32 = 11.0;

fn get_map(
    wgpu_app: &WgpuApp,
    map: &mut Vec<Sprite>,
) -> Result<(), Box<dyn std::error::Error>>{

    let file = File::open("map.txt")?;
    let reader = BufReader::new(file);


    let mut j = 0;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        for i in 0..parts.len(){

            let (tex_path, tex_pos, tex_cut) = match parts[i] {
                "0" => ("tex/grass.png", [0,0], [2,2]),
                "1" => ("tex/floor.png", [0,0], [2,2]),
                "2" => ("tex/wall.png",  [0,0], [2,5]),
                "3" => ("tex/wall.png",  [0,1], [2,5]),
                
                "4" => ("tex/wall.png",  [1,0], [2,5]), //door 01
                "5" => ("tex/wall.png",  [1,1], [2,5]), //door 02

                "6" => ("tex/wall.png",  [0,2], [2,5]), //wall left
                "7" => ("tex/wall.png",  [1,2], [2,5]), //wall right

                "8" => ("tex/wall.png",  [0,3], [2,5]),
                "8.1" => ("tex/wall.png",  [0,4], [2,5]),

                "9" => ("tex/wall.png",  [1,3], [2,5]),
                "9.1" => ("tex/wall.png",  [1,4], [2,5]),
                _ =>   ("tex/floor.png", [0,0], [2,2]),
            };

            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;

            let mut block: Sprite = Sprite::new(&wgpu_app.device, &wgpu_app.queue, tex_path, tex_pos, tex_cut);
            block.translation = [x, y, 0.0, 1.0];
            block.build_buffers(&wgpu_app.device);

            map.push(block);
        }

        j += 1;
    }

    Ok(())
}
