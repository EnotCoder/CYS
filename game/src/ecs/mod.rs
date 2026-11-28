// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Игровой ECS: адаптер, игровые компоненты, группы, размещение и частицы.
//  Примитивы спрайтов приходят из cys-engine.
// ========================================================================

pub mod adapter;
pub mod components;
pub mod cursor;
pub mod group;
pub mod placement;

pub mod particles {
    pub use crate::particles::*;
}

pub use cys_engine::ecs::{factory, Sprite, SpriteRenderData};
pub use components::*;
pub use adapter::EcsAdapter;
