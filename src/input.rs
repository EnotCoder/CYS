use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;
use std::cell::Cell;
use std::time::{Instant, Duration};

use crate::{EcsAdapter, Slot, add, remove};
use specs::Entity;

// === Константы ===
/// Задержка между перемещениями курсора (в миллисекундах)
const MOVE_DELAY: Duration = Duration::from_millis(200);
/// Мин. и макс. приближение карты
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 0.8;
/// Шаг зума за одно колёсико / нажатие
const ZOOM_STEP: f32 = 0.2;
/// Границы игрового поля (grid 9x9: -4..5)
const GRID_MIN: f32 = -4.0;
const GRID_MAX: f32 = 4.0;
/// Скорость движения курсора (в клетках)
const CURSOR_SPEED: f32 = 1.0;

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
    cursor_entity: Entity,
    icon_button: Entity,
    icons_slot_cursor: Entity,
) -> (i32, i32, f32) {
    let mut new_size = map_size;
    let mut new_mode = mode;
    let mut new_act_slot = act_slot;

    // 1. Зум (мышь + клавиатура)
    new_size = handle_zoom(input, new_size);

    // 2. Действия (поставить / удалить)
    if input.key_pressed(KeyCode::KeyF) {
        match mode {
            1 => add(ecs, slots, act_slot, cursor_entity),
            2 => {
                remove(ecs, cursor_entity);
            }
            _ => {}
        }
    }

    // 3. Переключение режимов (Tab)
    if input.key_pressed(KeyCode::Tab) {
        new_mode = cycle_mode(new_mode, ecs, cursor_entity, icon_button);
    }

    // 4. Переключение слота (Q)
    if input.key_pressed(KeyCode::KeyQ) {
        new_act_slot = cycle_slot(new_act_slot, slots, ecs, icons_slot_cursor);
    }

    // 5. Движение курсора (WASD)
    handle_movement(input, ecs, cursor_entity, new_mode, slots, new_act_slot);

    (new_act_slot, new_mode, new_size)
}

// ========================================================================
//  Зум: колёсико мыши + клавиши K/L
// ========================================================================
fn handle_zoom(input: &WinitInputHelper, current: f32) -> f32 {
    // Мышь
    let scroll = input.scroll_diff();
    if scroll.1 > 0.0 && current < MAX_ZOOM {
        return (current + ZOOM_STEP).min(MAX_ZOOM);
    }
    if scroll.1 < 0.0 && current > MIN_ZOOM {
        return (current - ZOOM_STEP).max(MIN_ZOOM);
    }

    // Клавиатура
    if input.key_pressed(KeyCode::KeyK) && current < MAX_ZOOM {
        return (current + ZOOM_STEP).min(MAX_ZOOM);
    }
    if input.key_pressed(KeyCode::KeyL) && current > MIN_ZOOM {
        return (current - ZOOM_STEP).max(MIN_ZOOM);
    }

    current
}

// ========================================================================
//  Циклическое переключение режимов: 0→1→2→0
// ========================================================================
fn cycle_mode(mode: i32, ecs: &mut EcsAdapter, cursor: Entity, icon: Entity) -> i32 {
    let new_mode = if mode == 2 { 0 } else { mode + 1 };

    // Меняем текстуру курсора
    let cursor_tex = match new_mode {
        0 => "tex/cursor/def_cursor.png",
        1 => "tex/cursor/cursor.png",
        2 => "tex/cursor/del_cursor.png",
        _ => unreachable!(),
    };
    ecs.update_sprite_texture(cursor, cursor_tex);

    // Меняем иконку режима в UI
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
//  Циклическое переключение слота инвентаря: 0→1→...→5→0
// ========================================================================
fn cycle_slot(
    slot: i32,
    slots: &mut [Slot],
    ecs: &mut EcsAdapter,
    cursor: Entity,
) -> i32 {
    // Деактивируем старый слот
    if (slot as usize) < slots.len() {
        slots[slot as usize].active = false;
    }

    let new_slot = if slot == 5 {
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
//  Движение курсора (WASD) с таймаутом MOVE_DELAY мс
// ========================================================================
fn handle_movement(
    input: &WinitInputHelper,
    ecs: &mut EcsAdapter,
    cursor: Entity,
    mode: i32,
    slots: &[Slot],
    act_slot: i32,
) {
    // Проверяем, прошло ли достаточно времени
    let now = Instant::now();
    let can_move = LAST_MOVE_TIME.with(|last| match last.get() {
        Some(t) => now.duration_since(t) >= MOVE_DELAY,
        None => true,
    });

    if !can_move {
        return;
    }

    let (x, y) = ecs.get_transform_position(cursor);
    let mut moved = false;

    if input.key_held(KeyCode::KeyW) && y < GRID_MAX {
        ecs.update_transform_position(cursor, x, y + CURSOR_SPEED);
        moved = true;
    }
    if input.key_held(KeyCode::KeyS) && y > GRID_MIN {
        ecs.update_transform_position(cursor, x, y - CURSOR_SPEED);
        moved = true;
    }
    if input.key_held(KeyCode::KeyA) && x > GRID_MIN {
        ecs.update_transform_position(cursor, x - CURSOR_SPEED, y);
        moved = true;
    }
    if input.key_held(KeyCode::KeyD) && x < GRID_MAX {
        ecs.update_transform_position(cursor, x + CURSOR_SPEED, y);
        moved = true;
    }

    if moved {
        LAST_MOVE_TIME.with(|last| last.set(Some(now)));
    }

    // Обновляем текстуру курсора (зелёный / красный) в режиме Build
    if mode == 1 {
        update_cursor_validity(ecs, cursor, slots, act_slot);
    }
}

// ========================================================================
//  Если режим Build — показываем зелёный/красный курсор
// ========================================================================
fn update_cursor_validity(ecs: &mut EcsAdapter, cursor: Entity, slots: &[Slot], act_slot: i32) {
    let (x, y) = ecs.get_transform_position(cursor);
    let slot = &slots[act_slot as usize];
    let is_carpet = matches!(slot.obj.name.as_str(), "carpet" | "red_carpet");
    let w = slot.obj.width;
    let h = slot.obj.height;

    if ecs.can_place_at(x as i32, y as i32, w, h, is_carpet) {
        ecs.update_sprite_texture(cursor, "tex/cursor/cursor.png");
    } else {
        ecs.update_sprite_texture(cursor, "tex/cursor/err cursor.png");
    }
}
