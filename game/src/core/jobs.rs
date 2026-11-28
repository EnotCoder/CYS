// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

pub use cys_engine::assets::jobs::*;

/// Предзагружает текстуры каталога вместе с базовыми текстурами движка.
pub fn prewarm_game_textures() {
    let paths: Vec<String> = crate::data::catalog()
        .objects
        .iter()
        .map(|object| object.path.to_string())
        .collect();
    cys_engine::assets::jobs::prewarm_paths(&paths);
}
