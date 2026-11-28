// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  cys-engine — платформенный слой: GPU, окна, ввод, тексты, виджеты, звук.
//  Движок не знает о магазине, сценах, Lua и сохранениях.
// ========================================================================

pub mod assets;
pub mod audio;
pub mod core;
pub mod ecs;
pub mod gfx;
pub mod input;
pub mod profiler;
pub mod text;
pub mod ui;
pub mod util;

pub use ecs::components::{SpriteComponent, Transform};
pub use ecs::{Sprite, SpriteRenderData};
pub use gfx::{buffers, constants, init, pipeline, render, texture};
pub use gfx::buffers::{init_buffers, DepthBuffer, LightData, Uniforms, Vertex};
pub use gfx::init::{Size, UiUniforms, WgpuApp};
pub use gfx::texture::Texture;
