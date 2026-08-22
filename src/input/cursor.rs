// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Курсор мыши: handle_mouse_movement (NDC → мир → клетка, с задержкой),
//  update_cursor_validity (текстура ок/ошибка), update_cursor_preview
//  (превью расстановки). LAST_MOVE_TIME — таймер задержки при движении.
// ========================================================================

use std::cell::Cell;
use std::time::{Instant, Duration};
use specs::Entity;
use crate::{EcsAdapter, data::{Slot, is_carpet_name, is_wall_decor_name, is_outdoor_name, is_flower_name}};
use crate::core::constants::*;
use crate::input::platform::InputSource;

// Запоминаем время последнего перемещения курсора на соседнюю клетку,
// чтобы задержать движение (случай перетаскивания по соседним тайлам)
thread_local! {
    static LAST_MOVE_TIME: Cell<Option<Instant>> = const { Cell::new(None) };
}

// Движение курсора за мышью: NDC -> мировые координаты -> клетка сетки
pub fn handle_mouse_movement(
    input: &dyn InputSource,
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

    // Перевод курсора из NDC в мировые координаты (с учётом зума и камеры)
    let (world_x, world_y) = crate::core::util::ndc_to_world(mouse_x, mouse_y, window_size, map_size, cam_x, cam_y);

    // Приводим к клетке сетки и ограничиваем границами карты
    let grid_x = (world_x + TILE_HALF).floor().clamp(CAMERA_MAP_MIN_X, CAMERA_MAP_MAX_X);
    let grid_y = (world_y + TILE_HALF).floor().clamp(CAMERA_MAP_MIN_Y, CAMERA_MAP_MAX_Y);

    // Если клетка под курсором не изменилась — действий нет
    let (cur_x, cur_y) = ecs.get_transform_position(cursor);
    if (cur_x - grid_x).abs() < EPSILON && (cur_y - grid_y).abs() < EPSILON {
        return;
    }

    // При перемещении на соседнюю клетку применяем задержку,
    // чтобы курсор не «прыгал» сквозь ряд клеток при быстром движении мыши
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

    // Ставим курсор в новую клетку
    ecs.update_transform_position(cursor, grid_x, grid_y);

    // В режиме расстановки сразу пересчитываем допустимость размещения
    if mode == 1 {
        update_cursor_validity(ecs, cursor, slots, act_slot);
    }
}

// Обновление текстуры курсора: разрешено/запрещено ставить предмет в этой клетке
// (учитываются ковры, настенный декор, уличные объекты и цветы)
pub fn update_cursor_validity(ecs: &mut EcsAdapter, cursor: Entity, slots: &[Slot], act_slot: i32) {
    let (x, y) = ecs.get_transform_position(cursor);
    let slot = &slots[act_slot as usize];
    let is_carpet = is_carpet_name(slot.obj.name);
    let is_wall_decor = is_wall_decor_name(slot.obj.name);
    let is_outdoor = is_outdoor_name(slot.obj.name);
    let is_flower = is_flower_name(slot.obj.name);

    if ecs.can_place_at(x as i32, y as i32, slot.obj.width, slot.obj.height, is_carpet, is_wall_decor, is_outdoor, is_flower) {
        ecs.update_sprite_texture(cursor, CURSOR_TEX[1]);
    } else {
        ecs.update_sprite_texture(cursor, CURSOR_ERR_TEX);
    }
}

// Превью объекта перед размещением (полупрозрачная копия с габаритами),
// показывается только в режиме расстановки (mode == 1)
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
    let is_flower = is_flower_name(slot.obj.name);
    let valid = ecs.can_place_at(cx as i32, cy as i32, slot.obj.width, slot.obj.height, is_carpet, is_wall_decor, is_outdoor, is_flower);
    // Передаём ECS размер, допустимость и текстуру — превью рисуется на месте курсора
    ecs.update_cursor_preview(
        cx, cy,
        slot.obj.width, slot.obj.height,
        valid,
        slot.obj.path,
        slot.obj.texture_frame,
        slot.obj.texture_count,
    );
}
