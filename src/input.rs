use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;
use std::cell::Cell;
use std::time::{Instant, Duration};

use crate::{EcsAdapter, Slot};
use crate::slot_object::{add, remove};
use specs::Entity;

// === Константы ===
/// Задержка между перемещениями курсора (в миллисекундах)
const MOVE_DELAY: Duration = Duration::from_millis(150);
/// Мин. и макс. приближение карты
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 1.0;
/// Шаг зума за одно колёсико / нажатие
const ZOOM_STEP: f32 = 0.2;
/// Границы игрового поля (grid 9x9: -4..5)
const GRID_MIN: f32 = -4.0;
const GRID_MAX: f32 = 4.0;
/// Коэффициент шейдера: world_to_ndc = 0.223 * map_size
const SHADER_SCALE: f32 = 0.223;

// Последнее время движения — используем Cell вместо unsafe static mut
thread_local! {
    static LAST_MOVE_TIME: Cell<Option<Instant>> = const { Cell::new(None) };
}

// ========================================================================
//  Основная функция ввода: вызывается каждый кадр
// ========================================================================
pub fn do_input(
    input: &WinitInputHelper,
    ecs: &mut EcsAdapter,
    slots: &mut Vec<Slot>,
    act_slot: i32,
    mode: i32,
    map_size: f32,
    window_size: (f32, f32),
    cursor_entity: Entity,
    icon_button: Entity,
    icons_slot_cursor: Entity,
    inventory_mode: bool,
) -> (i32, i32, f32) {
    let mut new_size = map_size;
    let mut new_mode = mode;
    let mut new_act_slot = act_slot;

    // 1. Зум (колёсико мыши + клавиши K/L)
    new_size = handle_zoom(input, new_size);

    // 2. Действия (поставить / удалить) — ЛКМ или F
    if input.key_pressed(KeyCode::KeyF) || input.mouse_pressed(0) {
        match mode {
            1 => add(ecs, slots, act_slot, cursor_entity),
            2 => { remove(ecs, cursor_entity); }
            _ => {}
        }
    }

    // 3. Переключение режимов (Tab)
    if input.key_pressed(KeyCode::Tab) {
        new_mode = cycle_mode(new_mode, ecs, cursor_entity, icon_button);
    }

    // 4. Переключение слота (только Q) — не в режиме инвентаря
    if !inventory_mode && input.key_pressed(KeyCode::KeyQ) {
        new_act_slot = cycle_slot(new_act_slot, slots, ecs, icons_slot_cursor);
    }

    // 5. Движение курсора за мышью
    handle_mouse_movement(input, ecs, cursor_entity, new_mode, slots, new_act_slot, new_size, window_size);

    // 6. Обновляем текстуру курсора при смене слота/режима (без движения)
    if new_mode == 1 {
        update_cursor_validity(ecs, cursor_entity, slots, new_act_slot);
    }

    // 7. Превью размера объекта (в режиме Build)
    update_cursor_preview(ecs, new_mode, slots, new_act_slot, cursor_entity);

    (new_act_slot, new_mode, new_size)
}

// ========================================================================
//  Зум: колёсико мыши + клавиши K/L
// ========================================================================
fn handle_zoom(input: &WinitInputHelper, current: f32) -> f32 {
    let scroll = input.scroll_diff();
    if scroll.1 > 0.0 && current < MAX_ZOOM {
        return (current + ZOOM_STEP).min(MAX_ZOOM);
    }
    if scroll.1 < 0.0 && current > MIN_ZOOM {
        return (current - ZOOM_STEP).max(MIN_ZOOM);
    }

    if input.key_pressed(KeyCode::KeyK) && current < MAX_ZOOM {
        return (current + ZOOM_STEP).min(MAX_ZOOM);
    }
    if input.key_pressed(KeyCode::KeyL) && current > MIN_ZOOM {
        return (current - ZOOM_STEP).max(MIN_ZOOM);
    }

    current
}

// ========================================================================
//  Переключение режимов: 0→1→2→0
// ========================================================================
fn cycle_mode(mode: i32, ecs: &mut EcsAdapter, cursor: Entity, icon: Entity) -> i32 {
    let new_mode = if mode == 2 { 0 } else { mode + 1 };

    let cursor_tex = match new_mode {
        0 => "tex/cursor/def_cursor.png",
        1 => "tex/cursor/cursor.png",
        2 => "tex/cursor/del_cursor.png",
        _ => unreachable!(),
    };
    ecs.update_sprite_texture(cursor, cursor_tex);

    let icon_tex = match new_mode {
        0 => "tex/ui/mode/standart_mode.png",
        1 => "tex/ui/mode/build_mode.png",
        2 => "tex/ui/mode/del_mode.png",
        _ => unreachable!(),
    };
    ecs.update_sprite_texture(icon, icon_tex);

    new_mode
}

// ========================================================================
//  Переключение слота: 0→1→...→5→0
// ========================================================================
fn cycle_slot(slot: i32, slots: &mut [Slot], ecs: &mut EcsAdapter, cursor: Entity) -> i32 {
    let max_slot = slots.len() as i32 - 1;
    if (slot as usize) < slots.len() {
        slots[slot as usize].active = false;
    }

    let new_slot = if slot >= max_slot {
        ecs.update_transform_position(cursor, -4.0, -4.0);
        0
    } else {
        let (x, _) = ecs.get_transform_position(cursor);
        ecs.update_transform_position(cursor, x + 1.0, -4.0);
        slot + 1
    };

    new_slot
}

// ========================================================================
//  Движение курсора за мышью
//
//  Конвертация пикселей → NDC → мировые координаты → сетка:
//    ndc_x = (mouse_x / win_w) * 2 - 1
//    ndc_y = 1 - (mouse_y / win_h) * 2
//    world = ndc / (0.223 * map_size)
//    grid  = round(world), clamped to [-4, 4]
// ========================================================================
fn handle_mouse_movement(
    input: &WinitInputHelper,
    ecs: &mut EcsAdapter,
    cursor: Entity,
    mode: i32,
    slots: &[Slot],
    act_slot: i32,
    map_size: f32,
    window_size: (f32, f32),
) {
    // Получаем позицию мыши в пикселях окна
    let Some((mouse_x, mouse_y)) = input.cursor() else { return };

    // Конвертируем в NDC [-1, 1], затем в мировые координаты
    let scale_factor = SHADER_SCALE * map_size;
    let world_x = ((mouse_x / window_size.0) * 2.0 - 1.0) / scale_factor;
    let world_y = (1.0 - (mouse_y / window_size.1) * 2.0) / scale_factor;

    // Округляем до ближайшей клетки сетки
    let grid_x = (world_x + 0.5).floor().clamp(GRID_MIN, GRID_MAX);
    let grid_y = (world_y + 0.5).floor().clamp(GRID_MIN, GRID_MAX);

    // Проверяем, изменилась ли клетка
    let (cur_x, cur_y) = ecs.get_transform_position(cursor);
    if (cur_x - grid_x).abs() < 0.01 && (cur_y - grid_y).abs() < 0.01 {
        return;
    }

    // Плавно или мгновенно: если клетка рядом — двигаем, если далеко — телепорт
    let dx = (grid_x - cur_x).abs();
    let dy = (grid_y - cur_y).abs();
    if dx <= 1.0 && dy <= 1.0 {
        // Соседняя клетка — с задержкой (плавное перемещение)
        let now = Instant::now();
        let can_move = LAST_MOVE_TIME.with(|last| match last.get() {
            Some(t) => now.duration_since(t) >= MOVE_DELAY,
            None => true,
        });
        if !can_move {
            return;
        }
        LAST_MOVE_TIME.with(|last| last.set(Some(now)));
    }
    // Если прыжок далеко — телепортируемся без задержки

    ecs.update_transform_position(cursor, grid_x, grid_y);

    // Обновляем цвет курсора в режиме Build
    if mode == 1 {
        update_cursor_validity(ecs, cursor, slots, act_slot);
    }
}

// ========================================================================
//  В режиме Build — зелёный/красный курсор
// ========================================================================
fn update_cursor_validity(ecs: &mut EcsAdapter, cursor: Entity, slots: &[Slot], act_slot: i32) {
    let (x, y) = ecs.get_transform_position(cursor);
    let slot = &slots[act_slot as usize];
    let is_carpet = matches!(slot.obj.name.as_str(), "carpet" | "red_carpet" | "green_carpet");

    if ecs.can_place_at(x as i32, y as i32, slot.obj.width, slot.obj.height, is_carpet) {
        ecs.update_sprite_texture(cursor, "tex/cursor/cursor.png");
    } else {
        ecs.update_sprite_texture(cursor, "tex/cursor/err cursor.png");
    }
}

// ========================================================================
//  update_cursor_preview: Показывает/скрывает превью размера объекта
// ========================================================================
fn update_cursor_preview(ecs: &mut EcsAdapter, mode: i32, slots: &[Slot], act_slot: i32, cursor: Entity) {
    if mode != 1 {
        ecs.clear_cursor_preview();
        return;
    }

    let slot = &slots[act_slot as usize];
    let (cx, cy) = ecs.get_transform_position(cursor);
    let is_carpet = matches!(slot.obj.name.as_str(), "carpet" | "red_carpet" | "green_carpet");
    let valid = ecs.can_place_at(cx as i32, cy as i32, slot.obj.width, slot.obj.height, is_carpet);
    ecs.update_cursor_preview(cx, cy, slot.obj.width, slot.obj.height, valid);
}
