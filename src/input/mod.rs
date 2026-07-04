pub mod camera;
pub mod interact;
pub mod cursor;

use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;
use specs::Entity;
use crate::{EcsAdapter, data::Slot};
use crate::constants::*;

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
    zoom_step: f32,
    window_size: (f32, f32),
    cursor_entity: Entity,
    icon_button: Entity,
    _icons_slot_cursor: Entity,
    inventory_mode: bool,
    cam_x: f32,
    cam_y: f32,
) -> (i32, i32, f32, bool) {
    let new_size = camera::handle_zoom(input, map_size, zoom_step);
    let mut new_mode = mode;
    let mut show_ilm = false;

    if input.key_pressed(KeyCode::KeyF) || input.mouse_pressed(winit::event::MouseButton::Left) {
        let skip = inventory_mode || input.cursor().map_or(false, |(mx, my)| {
            let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
            let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            let on_slot = (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < slots.len() as i32;
            let on_icons = (wy - SLOT_BAR_Y).abs() < TILE_HALF
                && ((wx - ICON_MODE_X).abs() < TILE_HALF || (wx - ACTIVE_X).abs() < TILE_HALF);
            on_slot || on_icons
        });
        if !skip {
            show_ilm = interact::do_interact(ecs, cursor_entity, new_mode, slots, act_slot);
        }
    }

    if input.key_pressed(KeyCode::Tab) {
        new_mode = interact::cycle_mode(new_mode, ecs, cursor_entity, icon_button);
    }

    cursor::handle_mouse_movement(input, ecs, cursor_entity, new_mode, slots, act_slot, new_size, window_size, cam_x, cam_y);

    if new_mode == 1 {
        cursor::update_cursor_validity(ecs, cursor_entity, slots, act_slot);
    }

    cursor::update_cursor_preview(ecs, new_mode, slots, act_slot, cursor_entity);

    (act_slot, new_mode, new_size, show_ilm)
}
