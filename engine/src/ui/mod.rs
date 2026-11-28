// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  ui — переиспользуемые виджеты и счётчик FPS.
//  Окна инвентаря/магазина/модов живут в крейте game.
// ========================================================================

#![allow(dead_code)]

pub mod components;
pub mod fps;

pub use crate::text::text_input;
pub use components::*;
