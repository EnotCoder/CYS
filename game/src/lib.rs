// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  cys-game — игровая логика: сцены, данные, NPC, Lua, сохранения и интерфейс.
//  Зависит только от cys-engine; обратной зависимости нет.
// ========================================================================

pub mod core;
pub mod data;
pub mod ecs;
pub mod input;
pub mod npc;
pub mod particles;
pub mod save;
pub mod scenes;
pub mod scripts;
pub mod ui;
pub mod weather_fx;

#[cfg(test)]
mod tests;

pub use cys_engine::audio;
pub use cys_engine::{
    DepthBuffer, LightData, Size, Sprite, SpriteRenderData, Texture, Transform, UiUniforms, Uniforms, Vertex,
    WgpuApp,
};
pub use ecs::components::*;
pub use ecs::{EcsAdapter, GroupComponent, GroupInfoResource};
