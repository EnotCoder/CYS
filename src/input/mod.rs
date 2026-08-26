// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Обработка ввода игрока: зум, движение курсора, клики/взаимодействия
// ========================================================================

pub mod camera;
pub mod interact;
pub mod cursor;
pub mod platform;

use winit::keyboard::KeyCode;
use specs::Entity;
use crate::{EcsAdapter, data::Slot};
use crate::core::constants::*;
use crate::input::platform::InputSource;

// ========================================================================
//  Основная точка входа: вызывается каждый кадр
// ========================================================================
pub fn do_input(
    input: &dyn InputSource,
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
) -> (i32, i32, f32, bool, i32) {
    // Зум колесом или клавишами K/L (в пределах min..max)
    let new_size = camera::handle_zoom(input, map_size, zoom_step);
    let mut new_mode = mode;
    let mut show_ilm = false;
    let mut switch_level = 0;

    // Следуем за мышью: курсор переводится из NDC в мировые координаты
    cursor::handle_mouse_movement(input, ecs, cursor_entity, new_mode, slots, act_slot, new_size, window_size, cam_x, cam_y);

    // Клик левой кнопкой или клавиша F — действие (ставить/уберать/взаимодействовать)
    if input.key_pressed(KeyCode::KeyF) || input.mouse_pressed(winit::event::MouseButton::Left) {
        // Клик по UI-зонам (слоты хотбара, иконки, кнопка инвентаря) не должен
        // вызывать действие в мире — в этом случае ввод пропускается
        let skip = inventory_mode || input.cursor().map_or(false, |(mx, my)| {
            let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
            let col = (wx - HOTBAR_X + TILE_HALF) as i32;
            let on_slot = (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < slots.len() as i32;
            let on_icons = (wy - SLOT_BAR_Y).abs() < TILE_HALF
                && ((wx - ICON_MODE_X).abs() < TILE_HALF || (wx - ACTIVE_X).abs() < TILE_HALF);
            let on_inv_btn = (wy - SLOT_BAR_Y).abs() < TILE_HALF && (wx - INV_BTN_X).abs() < TILE_HALF;
            on_slot || on_icons || on_inv_btn
        });
        if !skip {
            // Переводим позицию курсора в клетку сетки мира (с учётом зума и камеры)
            let (gx, gy) = input.cursor().map_or((-99, -99), |(mx, my)| {
                let (wx, wy) = crate::core::util::ndc_to_world(mx, my, window_size, new_size, cam_x, cam_y);
                ((wx + TILE_HALF).floor() as i32, (wy + TILE_HALF).floor() as i32)
            });
            // Выполняем действие в зависимости от режима (камера/курсор/ластик)
            let interact_result = interact::do_interact(ecs, gx, gy, new_mode, slots, act_slot);
            // 1 — показать подсветку (например, аркадный автомат), 2 — смена уровня
            show_ilm = interact_result == 1;
            switch_level = interact_result;
        }
    }

    // Tab — переключение режима (0: просмотр/взаимодействие, 1: расстановка, 2: удаление)
    if input.key_pressed(KeyCode::Tab) {
        new_mode = interact::cycle_mode(new_mode, ecs, cursor_entity, icon_button);
    }

    // В режиме расстановки проверяем, можно ли ставить предмет в клетку под курсором
    if new_mode == 1 {
        cursor::update_cursor_validity(ecs, cursor_entity, slots, act_slot);
    }

    // Обновляем прозрачный превью объекта перед курсором
    cursor::update_cursor_preview(ecs, new_mode, slots, act_slot, cursor_entity);

    // Возвращаем: активный слот, режим, новый зум, флаг подсветки, запрос смены уровня
    (act_slot, new_mode, new_size, show_ilm, switch_level)
}
