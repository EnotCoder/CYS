use std::cell::Cell;
use std::time::{Instant, Duration};
use winit_input_helper::WinitInputHelper;
use specs::Entity;
use crate::{EcsAdapter, Slot};
use crate::slot_object::{is_carpet_name, is_wall_decor_name, is_outdoor_name};
use crate::constants::*;

thread_local! {
    static LAST_MOVE_TIME: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub fn handle_mouse_movement(
    input: &WinitInputHelper,
    ecs: &mut EcsAdapter,
    cursor: Entity,
    mode: i32,
    slots: &[Slot],
    act_slot: i32,
    map_size: f32,
    window_size: (f32, f32),
    cam_x: f32,
    cam_y: f32,
) {
    let Some((mouse_x, mouse_y)) = input.cursor() else { return };

    let (world_x, world_y) = crate::util::ndc_to_world(mouse_x, mouse_y, window_size, map_size, cam_x, cam_y);

    let grid_x = (world_x + TILE_HALF).floor().clamp(CAMERA_MAP_MIN_X, CAMERA_MAP_MAX_X);
    let grid_y = (world_y + TILE_HALF).floor().clamp(CAMERA_MAP_MIN_Y, CAMERA_MAP_MAX_Y);

    let (cur_x, cur_y) = ecs.get_transform_position(cursor);
    if (cur_x - grid_x).abs() < EPSILON && (cur_y - grid_y).abs() < EPSILON {
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

pub fn update_cursor_validity(ecs: &mut EcsAdapter, cursor: Entity, slots: &[Slot], act_slot: i32) {
    let (x, y) = ecs.get_transform_position(cursor);
    let slot = &slots[act_slot as usize];
    let is_carpet = is_carpet_name(slot.obj.name);
    let is_wall_decor = is_wall_decor_name(slot.obj.name);
    let is_outdoor = is_outdoor_name(slot.obj.name);

    if ecs.can_place_at(x as i32, y as i32, slot.obj.width, slot.obj.height, is_carpet, is_wall_decor, is_outdoor) {
        ecs.update_sprite_texture(cursor, CURSOR_TEX[1]);
    } else {
        ecs.update_sprite_texture(cursor, CURSOR_ERR_TEX);
    }
}

pub fn update_cursor_preview(ecs: &mut EcsAdapter, mode: i32, slots: &[Slot], act_slot: i32, cursor: Entity) {
    if mode != 1 {
        ecs.clear_cursor_preview();
        return;
    }

    let slot = &slots[act_slot as usize];
    let (cx, cy) = ecs.get_transform_position(cursor);
    let is_carpet = is_carpet_name(slot.obj.name);
    let is_wall_decor = is_wall_decor_name(slot.obj.name);
    let is_outdoor = is_outdoor_name(slot.obj.name);
    let valid = ecs.can_place_at(cx as i32, cy as i32, slot.obj.width, slot.obj.height, is_carpet, is_wall_decor, is_outdoor);
    ecs.update_cursor_preview(
        cx, cy,
        slot.obj.width, slot.obj.height,
        valid,
        slot.obj.path,
        slot.obj.texture_frame,
        slot.obj.texture_count,
    );
}
