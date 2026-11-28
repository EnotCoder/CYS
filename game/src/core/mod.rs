// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  game::core — совместимый фасад на.engine и игровые дополнения:
//  утилиты каталога, предзагрузка текстур, паки и моды.
// ========================================================================

pub use cys_engine::{Size, UiUniforms, WgpuApp};
pub use cys_engine::core::{
    asset, buffers, constants, frame_profiler, init, pipeline, render, texture,
};

pub mod jobs;
pub mod shader_pack;
pub mod texture_pack;
pub mod util;

use cys_engine::assets::texture_pack::TextureLayerProvider;

struct ModLayers;

impl TextureLayerProvider for ModLayers {
    fn layers(&self) -> Vec<String> {
        crate::scripts::mods::texture_layers()
    }

    fn root(&self, mod_id: &str) -> String {
        crate::scripts::mods::mod_root(mod_id)
    }
}

/// Подключает игровые моды к движковому резолверу текстур и применяет паки.
pub fn init() {
    cys_engine::assets::texture_pack::set_layer_provider(Box::new(ModLayers));
    texture_pack::init();
    shader_pack::init();
}
