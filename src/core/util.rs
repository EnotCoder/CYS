// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use std::hash::{Hash, Hasher};
use crate::core::constants::SHADER_SCALE;

// Ключ для кэша спрайтов: комбинация слоя, текстуры, кадра/атласа и масштаба
pub fn sprite_cache_key(layer: &str, path: &str, frame: [i32; 2], atlas: [i32; 2], scale: f32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    layer.hash(&mut hasher);
    path.hash(&mut hasher);
    frame.hash(&mut hasher);
    atlas.hash(&mut hasher);
    // scale.to_bits() — чтобы дробное значение хешировалось воспроизводимо
    scale.to_bits().hash(&mut hasher);
    hasher.finish()
}

// Возвращает коэффициент easeOutBack для «поп»-анимаций (перелёт + отскок).
// t в 0..=1. Используется в UI-анимациях (появление инвентаря, пульсы).
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

// Перевод координат курсора из NDC [-1..1] в мировые координаты игры.
// Используется при отслеживании мыши: масштаб зависит от зума (map_size)
// и положения камеры (cam_x/cam_y); ось Y в NDC направлена вверх
pub fn ndc_to_world(mx: f32, my: f32, window_size: (f32, f32), map_size: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let aspect = window_size.0 / window_size.1;
    let scale = SHADER_SCALE * map_size;
    // X нужно скорректировать на соотношение сторон, чтобы не было растяжения
    let wx = ((mx / window_size.0) * 2.0 - 1.0) * aspect / scale + cam_x;
    let wy = (1.0 - (my / window_size.1) * 2.0) / scale + cam_y;
    (wx, wy)
}

// Индекс предмета в списке текущей вкладки по координатам сетки (row/col).
// Сетка нумеруется сверху вниз, поэтому строки переворачиваются
pub fn inventory_index(row: i32, col: i32) -> i32 {
    (crate::core::constants::INVENTORY_ROWS - 1 - row) * crate::core::constants::INVENTORY_COLS + col
}

// Путь к иконке предмета в хотбаре: ищем имя в категориях, чтобы выбрать подкаталог
pub fn slot_icon_path(name: &str) -> String {
    use crate::core::constants::*;
    let subdir = if INV_REGULAR.contains(&name) { "regular" }
        else if INV_CARPETS.contains(&name) { "carpets" }
        else if INV_WALLDECOR.contains(&name) { "walldecor" }
        else if INV_OUTDOOR.contains(&name) { "outdoor" }
        // Незнакомое имя — берём иконку напрямую из каталога предметов
        else { return format!("{}{}.png", TEX_UI_ICON_SLOTS_MAP_DIR, name) };
    format!("{}{}/{}.png", TEX_UI_ICON_SLOTS_OBJECT_DIR, subdir, name)
}
