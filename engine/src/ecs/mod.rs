// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  ecs — примитивы спрайтов: компоненты и фабрика сущностей.
//  Игровые компоненты и адаптер живут в крейте game.
// ========================================================================

pub mod components;
pub mod factory;

pub use components::*;
pub use crate::gfx::sprite::{Sprite, SpriteRenderData};
