// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

pub use cys_engine::assets::shader_pack::*;

/// Применяет шейдер из сохранённых настроек при запуске.
pub fn init() {
    let id = crate::save::SETTINGS.lock().unwrap().shader.clone();
    cys_engine::assets::shader_pack::init_with(id.as_deref());
}
