// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  UI: система интерфейса — компоненты (Panel/Button/Checkbox/Slider),
//  хотбар, настройки, FPS-счётчик и TextRenderer.
// ========================================================================

#![allow(dead_code)]

pub mod components;
pub mod fps;
pub mod inventory;
pub mod settings;
pub mod shop;
pub mod system;
pub mod text_input;
pub mod text_renderer;
pub mod weather;

pub use components::*;
pub use system::*;
