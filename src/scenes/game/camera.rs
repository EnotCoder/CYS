// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  camera.rs — управление камерой в игровой сцене
// ========================================================================
//  Камера перемещается зажатой средней кнопкой мыши (drag) или стрелками
//  клавиатуры. Ограничивается видимой областью карты, чтобы не уходить
//  за пределы уровня.
// ========================================================================

use winit::keyboard::KeyCode;
use crate::core::constants::*;
use crate::input::platform::InputSource;
use super::GameScene;

impl GameScene {
    /// Обновляет смещение камеры: drag средней кнопкой мыши + стрелки клавиатуры.
    /// Ограничивает позицию видимой областью карты.
    pub fn update_camera(&mut self, input: &dyn InputSource, window_size: (f32, f32), dt: f64) {
        // Расчёт видимой области карты для ограничения движения камеры
        let aspect = window_size.0 / window_size.1;
        let vis_w = 2.0 * aspect / (SHADER_SCALE * self.map_size);
        let vis_h = 2.0 / (SHADER_SCALE * self.map_size);
        let cam_min_x = CAMERA_MAP_MIN_X + vis_w / 2.0;
        let cam_max_x = CAMERA_MAP_MAX_X - vis_w / 2.0;
        let cam_min_y = CAMERA_MAP_MIN_Y + vis_h / 2.0;
        let cam_max_y = CAMERA_MAP_MAX_Y - vis_h / 2.0;
        // При широком окне или сильном отдалении видимая область шире
        // допустимого диапазона камеры — границы инвертируются (min > max) и
        // .clamp() паникует. В таком случае фиксируем камеру по центру карты.
        let cam_center_x = (CAMERA_MAP_MIN_X + CAMERA_MAP_MAX_X) / 2.0;
        let cam_center_y = (CAMERA_MAP_MIN_Y + CAMERA_MAP_MAX_Y) / 2.0;
        let (cam_min_x, cam_max_x) = if cam_max_x >= cam_min_x { (cam_min_x, cam_max_x) } else { (cam_center_x, cam_center_x) };
        let (cam_min_y, cam_max_y) = if cam_max_y >= cam_min_y { (cam_min_y, cam_max_y) } else { (cam_center_y, cam_center_y) };

        let step = CAMERA_SPEED * (dt as f32);

        // Камера: перемещение зажатой средней кнопкой мыши
        // (пропускаем, пока открыт магазин — палец используется для скролла каталога)
        if input.mouse_held(winit::event::MouseButton::Middle) && !self.shop.open {
            let sensitivity = 0.01;
            let (dx, dy) = input.cursor_diff();
            self.camera_offset_x = (self.camera_offset_x - dx * sensitivity).clamp(cam_min_x, cam_max_x);
            self.camera_offset_y = (self.camera_offset_y + dy * sensitivity).clamp(cam_min_y, cam_max_y);
        }

        // Камера: перемещение стрелками клавиатуры
        if input.key_held(KeyCode::ArrowLeft) {
            self.camera_offset_x = (self.camera_offset_x - step).max(cam_min_x);
        }
        if input.key_held(KeyCode::ArrowRight) {
            self.camera_offset_x = (self.camera_offset_x + step).min(cam_max_x);
        }
        if input.key_held(KeyCode::ArrowDown) {
            self.camera_offset_y = (self.camera_offset_y - step).max(cam_min_y);
        }
        if input.key_held(KeyCode::ArrowUp) {
            self.camera_offset_y = (self.camera_offset_y + step).min(cam_max_y);
        }
        self.camera_offset_x = self.camera_offset_x.clamp(cam_min_x, cam_max_x);
        self.camera_offset_y = self.camera_offset_y.clamp(cam_min_y, cam_max_y);
    }
}
