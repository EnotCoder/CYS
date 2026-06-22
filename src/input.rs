use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;
use std::cell::Cell;
use std::time::{Instant, Duration};

use crate::{EcsAdapter, Slot};
use crate::slot_object::{add, remove, is_carpet_name};
use crate::constants::*;
use specs::Entity;

thread_local! {
    static LAST_MOVE_TIME: Cell<Option<Instant>> = const { Cell::new(None) };
}

// ========================================================================
//  Основная точка входа: вызывается каждый кадр
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
    _icons_slot_cursor: Entity,
    inventory_mode: bool,
) -> (i32, i32, f32) {
    let mut new_size = map_size;
    let mut new_mode = mode;
    let aspect = window_size.0 / window_size.1;

    new_size = handle_zoom(input, new_size);

    if input.key_pressed(KeyCode::KeyF) || input.mouse_pressed(0) {
        let skip = inventory_mode || input.cursor().map_or(false, |(mx, my)| {
            let scale_factor = SHADER_SCALE;
            let wx = ((mx / window_size.0) * 2.0 - 1.0) * aspect / scale_factor;
            let wy = (1.0 - (my / window_size.1) * 2.0) / scale_factor;
            let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < slots.len() as i32
        });
        if !skip {
            match mode {
                1 => add(ecs, slots, act_slot, cursor_entity),
                2 => { remove(ecs, cursor_entity); }
                _ => {}
            }
        }
    }

    if input.key_pressed(KeyCode::Tab) {
        new_mode = cycle_mode(new_mode, ecs, cursor_entity, icon_button);
    }

    handle_mouse_movement(input, ecs, cursor_entity, new_mode, slots, act_slot, new_size, window_size, aspect);

    if new_mode == 1 {
        update_cursor_validity(ecs, cursor_entity, slots, act_slot);
    }

    update_cursor_preview(ecs, new_mode, slots, act_slot, cursor_entity);

    (act_slot, new_mode, new_size)
}

// ========================================================================
//  Зум
// ========================================================================
fn handle_zoom(input: &WinitInputHelper, current: f32) -> f32 {
    let scroll = input.scroll_diff();
    if scroll.1 > 0.0 && current < ZOOM_MAX {
        return (current + ZOOM_STEP).min(ZOOM_MAX);
    }
    if scroll.1 < 0.0 && current > ZOOM_MIN {
        return (current - ZOOM_STEP).max(ZOOM_MIN);
    }
    if input.key_pressed(KeyCode::KeyK) && current < ZOOM_MAX {
        return (current + ZOOM_STEP).min(ZOOM_MAX);
    }
    if input.key_pressed(KeyCode::KeyL) && current > ZOOM_MIN {
        return (current - ZOOM_STEP).max(ZOOM_MIN);
    }
    current
}

// ========================================================================
//  Переключение режимов: 0→1→2→0
// ========================================================================
fn cycle_mode(mode: i32, ecs: &mut EcsAdapter, cursor: Entity, icon: Entity) -> i32 {
    let new_mode = if mode == 2 { 0 } else { mode + 1 };
    ecs.update_sprite_texture(cursor, CURSOR_TEX[new_mode as usize]);
    ecs.update_sprite_texture(icon, MODE_ICON_TEX[new_mode as usize]);
    new_mode
}

// ========================================================================
//  Переключение слота на панели
// ========================================================================
// ========================================================================
//  Движение курсора за мышью
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
    aspect: f32,
) {
    let Some((mouse_x, mouse_y)) = input.cursor() else { return };

    let scale_factor = SHADER_SCALE * map_size;
    let world_x = ((mouse_x / window_size.0) * 2.0 - 1.0) * aspect / scale_factor;
    let world_y = (1.0 - (mouse_y / window_size.1) * 2.0) / scale_factor;

    let grid_x = (world_x + TILE_HALF).floor().clamp(GRID_MIN_X, GRID_MAX_X);
    let grid_y = (world_y + TILE_HALF).floor().clamp(GRID_MIN_Y, GRID_MAX_Y);

    let (cur_x, cur_y) = ecs.get_transform_position(cursor);
    if (cur_x - grid_x).abs() < 0.01 && (cur_y - grid_y).abs() < 0.01 {
        return;
    }

    let dx = (grid_x - cur_x).abs();
    let dy = (grid_y - cur_y).abs();
    if dx <= 1.0 && dy <= 1.0 {
        let now = Instant::now();
        let can_move = LAST_MOVE_TIME.with(|last| match last.get() {
            Some(t) => now.duration_since(t) >= Duration::from_millis(CURSOR_MOVE_DELAY_MS),
            None => true,
        });
        if !can_move { return; }
        LAST_MOVE_TIME.with(|last| last.set(Some(now)));
    }

    ecs.update_transform_position(cursor, grid_x, grid_y);

    if mode == 1 {
        update_cursor_validity(ecs, cursor, slots, act_slot);
    }
}

// ========================================================================
//  Валидность размещения (зелёный/красный курсор)
// ========================================================================
fn update_cursor_validity(ecs: &mut EcsAdapter, cursor: Entity, slots: &[Slot], act_slot: i32) {
    let (x, y) = ecs.get_transform_position(cursor);
    let slot = &slots[act_slot as usize];
    let is_carpet = is_carpet_name(slot.obj.name);

    if ecs.can_place_at(x as i32, y as i32, slot.obj.width, slot.obj.height, is_carpet) {
        ecs.update_sprite_texture(cursor, CURSOR_TEX[1]);
    } else {
        ecs.update_sprite_texture(cursor, CURSOR_ERR_TEX);
    }
}

// ========================================================================
//  Превью размера объекта
// ========================================================================
fn update_cursor_preview(ecs: &mut EcsAdapter, mode: i32, slots: &[Slot], act_slot: i32, cursor: Entity) {
    if mode != 1 {
        ecs.clear_cursor_preview();
        return;
    }

    let slot = &slots[act_slot as usize];
    let (cx, cy) = ecs.get_transform_position(cursor);
    let is_carpet = is_carpet_name(slot.obj.name);
    let valid = ecs.can_place_at(cx as i32, cy as i32, slot.obj.width, slot.obj.height, is_carpet);
    ecs.update_cursor_preview(cx, cy, slot.obj.width, slot.obj.height, valid);
}
