// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Compatible façade over the new engine layout. Game code keeps using
// `core::constants`, `core::asset`, `core::init`, etc. while the files live in
// `gfx/` and `assets/`.

pub use crate::assets::{asset, jobs, shader_pack, texture_pack};
pub use crate::gfx::{buffers, constants, init, pipeline, render, texture};
pub use crate::profiler as frame_profiler;
pub use crate::util;
