// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Утилиты, которым нужна игроваякаталогизация предметов.

pub use cys_engine::util::*;

use cys_engine::gfx::constants::{
    INVENTORY_COLS, INVENTORY_ROWS, INV_CARPETS, INV_LIGHT, INV_OUTDOOR, INV_REGULAR,
    INV_WALLDECOR, TEX_UI_ICON_SLOTS_MAP_DIR, TEX_UI_ICON_SLOTS_OBJECT_DIR,
};

// Индекс предмета в списке текущей вкладки по координатам сетки (row/col).
// Сетка нумеруется сверху вниз, поэтому строки переворачиваются.
pub fn inventory_index(row: i32, col: i32) -> i32 {
    (INVENTORY_ROWS - 1 - row) * INVENTORY_COLS + col
}

// Путь к иконке предмета в хотбаре: ищем имя в категориях, чтобы выбрать подкаталог.
pub fn slot_icon_path(name: &str) -> String {
    if let Some(meta) = crate::data::item_meta(name) {
        return meta.icon.to_string();
    }
    let subdir = if INV_REGULAR.contains(&name) {
        "regular"
    } else if INV_CARPETS.contains(&name) {
        "carpets"
    } else if INV_WALLDECOR.contains(&name) {
        "walldecor"
    } else if INV_OUTDOOR.contains(&name) {
        "outdoor"
    } else if INV_LIGHT.contains(&name) {
        "light"
    } else {
        return format!("{TEX_UI_ICON_SLOTS_MAP_DIR}{name}.png");
    };
    format!("{TEX_UI_ICON_SLOTS_OBJECT_DIR}{subdir}/{name}.png")
}

/// Размер иконки предмета внутри ячейки инвентаря (1.0 — вся ячейка).
pub fn slot_icon_size(name: &str) -> [f32; 2] {
    crate::data::item_meta(name)
        .map(|meta| meta.icon_size)
        .unwrap_or([1.0, 1.0])
}
