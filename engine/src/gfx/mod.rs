// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  gfx — графика: инициализация wgpu, буферы, пайплайны, спрайты и рендер.
// ========================================================================

pub mod buffers;
pub mod constants;
pub mod init;
pub mod pipeline;
pub mod render;
pub mod sprite;
pub mod texture;

pub use sprite::{Sprite, SpriteRenderData};
