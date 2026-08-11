// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use winit_input_helper::WinitInputHelper;
use winit::keyboard::KeyCode;
use crate::constants::*;

// Приближение/отдаление камеры: колесо мыши или клавиши K/L
// Зум ограничен диапазоном [ZOOM_MIN, ZOOM_MAX], шаг задаётся извне
pub fn handle_zoom(input: &WinitInputHelper, current: f32, zoom_step: f32) -> f32 {
    let scroll = input.scroll_diff();
    // Колесо вверх — приближение (не выше максимума)
    if scroll.1 > 0.0 && current < ZOOM_MAX {
        return (current + zoom_step).min(ZOOM_MAX);
    }
    // Колесо вниз — отдаление (не ниже минимума)
    if scroll.1 < 0.0 && current > ZOOM_MIN {
        return (current - zoom_step).max(ZOOM_MIN);
    }
    // Клавишная альтернатива колесу
    if input.key_pressed(KeyCode::KeyK) && current < ZOOM_MAX {
        return (current + zoom_step).min(ZOOM_MAX);
    }
    if input.key_pressed(KeyCode::KeyL) && current > ZOOM_MIN {
        return (current - zoom_step).max(ZOOM_MIN);
    }
    // Никакого изменения — возвращаем текущее значение
    current
}
