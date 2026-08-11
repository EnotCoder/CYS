// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Модуль ECS — реестр подсистем Entity-Component-System (specs).
//  Собирает компоненты хранилища, фабрику спрайтов, логику размещения,
//  курсор/предпросмотр, группировку объектов и адаптер для рендера.
// ========================================================================

pub mod adapter;
pub mod components;
pub mod cursor;
pub mod factory;
pub mod group;
pub mod placement;
pub mod sprite;

// Пере-экспорт: наружу отдаём адаптер и типы компонентов, чтобы
// остальной код работал через единый корневой путь crate::ecs::*.
pub use adapter::*;
pub use components::*;
pub use sprite::Sprite;
