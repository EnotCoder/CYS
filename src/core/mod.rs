// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  core: низкоуровневые компоненты wgpu-пайплайна — «ядро» игры.
//
//  Модуль собирает воедино "железо" рендера:
//    buffers.rs  — структуры данных (вершины, uniform'ы) и макеты биндингов;
//    init.rs     — инициализация WgpuApp: surface, device, буферы, пайплайны;
//    texture.rs  — загрузка и создание текстур/сэмплеров;
//    render.rs   — отрисовка кадра по слоям (map / transparent / ui);
//    constants.rs — все константы игры;
//    util.rs     — вспомогательные утилиты (кэш-ключи, координаты, иконки).
// ========================================================================

pub mod buffers;
pub mod init;
pub mod pipeline;
pub mod render;
pub mod texture;
pub mod constants;
pub mod util;

// Продляем наружу всё содержимое подмодулей, чтобы работать было удобнее.
pub use buffers::*;
pub use init::*;
pub use render::*;
pub use texture::*;
