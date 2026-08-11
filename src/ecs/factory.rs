// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::{World, Entity, Builder, WorldExt};
use std::sync::Arc;
use crate::ecs::components::{Transform, SpriteComponent};

// Создаёт сущность с компонентами Transform + SpriteComponent.
// Это единая точка создания спрайтов: текстура задаётся без привязки к кадру
// атласа здесь — кадр/масштаб передаются как есть.
pub fn create_sprite(
    world: &mut World, x: f32, y: f32, z: f32,
    texture_path: &str, frame: [i32; 2], count: [i32; 2],
    scale: f32, alpha: f32,
) -> Entity {
    world
        .create_entity()
        .with(Transform { position: [x, y, z] })
        .with(SpriteComponent {
            texture_path: Arc::from(texture_path),
            texture_frame: frame,
            texture_count: count,
            scale,
            alpha,
            animated: false,
            frame_paths: Vec::new(),
            current_frame: 0,
        })
        .build()
}


