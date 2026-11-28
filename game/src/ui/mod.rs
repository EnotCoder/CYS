// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  UI игры: окна инвентаря, настроек, магазина, модов и погоды.
//  Виджеты, FPS и текстовый ввод приходят из cys-engine.
// ========================================================================

#![allow(dead_code)]

pub use cys_engine::ui::{components, fps, text_input};

pub mod anim;
pub mod inventory;
pub mod mods;
pub mod settings;
pub mod shop;
pub mod system;
pub mod text_renderer;
pub mod weather;

pub use components::*;
pub use system::*;
