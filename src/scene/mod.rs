// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Модуль сцен — меню и игровой уровень
// ========================================================================
//  Здесь объявлены трейт Scene, реестр сцен SceneManager, главное меню
//  и игровая сцена (в подмодуле game/ — цикл день/ночь, HUD и покупатели).

pub mod scene_trait;
pub mod scene_manager;
pub mod menu_scene;
pub mod game;

pub use scene_trait::*;
pub use scene_manager::*;
pub use menu_scene::MenuScene;
pub use game::GameScene;
