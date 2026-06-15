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

use api_components::*;
use sprite_manager::*;
use slot_object::*;
use input::*;
use ecs::*;

use specs::{WorldExt, Builder};

// === Константы ===
/// Смещение мира. Карта центрируется с помощью этих констант
/// (координаты в файле map.txt: i=0..N, j=0..M -> мир: [-11 + i, 11 - j])
const WORLD_OFFSET_X: f32 = -11.0;
const WORLD_OFFSET_Y: f32 = 11.0;

/// Пустой Uniforms — используется каждый кадр для сброса
const EMPTY_UNIFORMS: Uniforms = Uniforms {
    translation: [0.0; 4],
    rotation: [0.0; 4],
    _padding: [0.0; 3],
};

#[tokio::main]
async fn main() {
    // === 1. Окно и EventLoop ===
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("CYS — Create your Shop")
        .with_inner_size(PhysicalSize::new(800, 800))
        .build(&event_loop)
        .unwrap();

    // === 2. WGPU инициализация ===
    let mut wgpu_app = WgpuApp::new(&window).await;
    let surface = wgpu_app
        .instance
        .create_surface(&window)
        .expect("Failed to create surface");

    // === 3. ECS (сущности) ===
    let mut ecs = EcsAdapter::new();

    // Курсор — z = 2.0 (слой над decor)
    let cursor_entity = ecs.add_cursor(0.0, 0.0, "tex/cursor/def_cursor.png");
    load_map_to_ecs(&mut ecs);

    // === 4. Слоты (инвентарь) ===
    let mut act_slot: i32 = 0;
    let mut slots: Vec<Slot> = get_slot_vec();
    let (icon_button, icons_slot_cursor) = get_uv_ecs(&mut ecs, &slots);

    // === 5. Главный цикл ===
    let mut input = winit_input_helper::WinitInputHelper::new();
    let mut mode: i32 = 0;
    let mut map_size: f32 = 0.8;

    let _ = event_loop.run(|event, event_loop_target| {
        // Размер окна (нужен для конвертации координат мыши)
        let window_size = (
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );

        // -- Ввод --
        if input.update(&event) {
            (act_slot, mode, map_size) = do_input(
                &input,
                &mut ecs,
                &mut slots,
                act_slot,
                mode,
                map_size,
                window_size,
                cursor_entity,
                icon_button,
                icons_slot_cursor,
            );
        }

        // -- Uniforms (каждый кадр) --
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.uniform_buffer, 0, bytemuck::cast_slice(&[EMPTY_UNIFORMS]));

        let size_data = Size { map_size };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));

        // UI uniform — константа 1.0
        let ui_uniforms = UiUniforms {
            size: 1.0,
            _padding: [0.0; 3],
        };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

        // -- Обработка событий --
        match event {
            // Закрытие окна
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => event_loop_target.exit(),

            // Запрос перерисовки
            Event::AboutToWait => window.request_redraw(),

            // Рендер
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let (map_sprites, carpet_sprites, decor_sprites, cursor_sprites, ui_sprites) =
                    ecs.get_sprites_by_layer();

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
                    &mut ecs.sprite_cache,
                );
            }

            // Изменение размера окна
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

// ========================================================================
//  load_map_to_ecs: Читает map.txt и создаёт ECS-сущности для тайлов карты
// ========================================================================
fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open("map.txt").expect("map.txt not found!");
    let reader = BufReader::new(file);

    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        for (i, token) in parts.iter().enumerate() {
            // Определяем тип тайла по коду из map.txt
            // "0" / "0.1" / "0.2" / "0.3" → grass.png
            // "1" → floor.png
            // "2".."9.1" → wall.png (variants 2..=9, 8.1, 9.1)
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

/// Возвращает данные для тайлов травы (атлас 3x2)
fn grass_frame(code: &str) -> (&str, [i32; 2], [i32; 2]) {
    match code {
        "0" => ("tex/grass.png", [0, 0], [3, 2]),
        "0.1" => ("tex/grass.png", [1, 0], [3, 2]),
        "0.2" => ("tex/grass.png", [0, 1], [3, 2]),
        "0.3" => ("tex/grass.png", [2, 1], [3, 2]),
        _ => ("tex/grass.png", [0, 0], [3, 2]),
    }
}

/// Возвращает данные для тайлов стен (атлас 2x5)
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

// ========================================================================
//  get_uv_ecs: Создаёт ECS-сущности для UI (иконка режима + инвентарь)
// ========================================================================
fn get_uv_ecs(ecs: &mut EcsAdapter, slots: &[Slot]) -> (specs::Entity, specs::Entity) {
    // Иконка текущего режима (справа вверху)
    let icon_mode = ecs.add_ui(4.0, -4.0, "tex/ui/mode/standart_mode.png");

    // 6 иконок слотов (снизу слева)
    for (i, slot) in slots.iter().enumerate() {
        ecs.add_ui(
            -4.0 + i as f32,
            -4.0,
            &format!("tex/ui/icon_slots/{}.png", slot.obj.name),
        );
    }

    // Курсор выбора слота
    let icons_slot_cursor = ecs.add_ui(-4.0, -4.0, "tex/ui/icon_slots/cursor.png");

    (icon_mode, icons_slot_cursor)
}
