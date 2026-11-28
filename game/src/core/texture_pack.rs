// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

pub use cys_engine::assets::texture_pack::*;

/// Применяет пак из сохранённых настроек при запуске.
pub fn init() {
    let id = crate::save::SETTINGS.lock().unwrap().texture_pack.clone();
    cys_engine::assets::texture_pack::init_with(id.as_deref());
}
