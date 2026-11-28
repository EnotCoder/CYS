// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Компоненты и ресурсы ECS: Transform, SpriteComponent, GroupComponent,
//  Rotation, ObjectTag, FoodStorage, FenceComponent; ресурсы мира
//  GroupInfoResource, TotalFood, Money, BusyCassas, BasementPlaced.
// ========================================================================

use specs::{Component, VecStorage};
use std::sync::Arc;

// Позиция сущности в мировых координатах; z задаёт слой (Z_MAP/Z_DECOR/...).
#[derive(Debug)]
pub struct Transform {
    pub position: [f32; 3],
}

impl Component for Transform {
    type Storage = VecStorage<Self>;
}

// SpriteComponent
// Отображаемая графика: путь к текстуре-атласу, выбранный кадр,
// количество кадров по осям атласа, масштаб и прозрачность.
#[derive(Debug)]
pub struct SpriteComponent {
    pub texture_path: Arc<str>,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub scale: f32,
    pub alpha: f32,
    pub animated: bool,
    pub frame_paths: Vec<String>,
    pub current_frame: i32,
    /// Скорость анимации этого спрайта (кадров в секунду).
    pub anim_fps: f32,
    /// Зацикливать ли набор кадров.
    pub anim_loop: bool,
    /// Накопленное время до следующего кадра.
    pub anim_timer: f64,
}

impl Component for SpriteComponent {
    type Storage = VecStorage<Self>;
}
