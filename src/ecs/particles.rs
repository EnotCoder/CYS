// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  particles — переиспользуемая система короткоживущих ECS-партиклов.
//  Симуляция живёт в обычных структурах и не требует GPU; в ECS частицы
//  попадают на Z_WEATHER только в момент синхронизации кадра.
// ========================================================================

use crate::core::asset;
use crate::core::constants::Z_WEATHER;
use crate::core::texture_pack;
use crate::core::util::{rand01, sprite_cache_key};
use crate::ecs::factory::create_sprite;
use crate::{EcsAdapter, Sprite, Texture};
use specs::Entity;
use std::collections::{HashMap, HashSet};

// Погода занимает до 170 dynamic-слотов, а лимит AMD RADV — 256 слотов.
pub const MAX_ACTIVE_PARTICLES: usize = 64;

const TEXTURE_SIZE: u32 = 4;
const FALLBACK_COLOR: [u8; 3] = [190, 190, 190];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BurstKind {
    Place,
    Remove,
}

#[derive(Clone, Copy, Debug)]
pub struct PlacementFxEvent {
    pub kind: BurstKind,
    pub x: f32,
    pub y: f32,
    pub width: i32,
    pub height: i32,
    pub color: [u8; 3],
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub struct ParticleSnapshot {
    pub kind: BurstKind,
    pub x: f32,
    pub y: f32,
    pub color: [u8; 3],
    pub alpha: f32,
}

struct Particle {
    kind: BurstKind,
    color: [u8; 3],
    path: String,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
    ttl: f32,
    entity: Option<Entity>,
}

// Доминирующий непрозрачный пиксель кадра предмета — его и используем как
// цвет партиклов. Учитываем активный текстур-пак и слои модов.
pub fn object_color(name: &str, path: &str, frame: [i32; 2], atlas: [i32; 2]) -> [u8; 3] {
    let path = texture_pack::object_texture(name, path);
    let frame = texture_pack::object_frame(name, frame);
    let atlas = texture_pack::object_atlas(name, atlas);
    let resolved = texture_pack::resolve(&path);
    let bytes = asset::load_bytes(&resolved).or_else(|_| asset::load_bytes(&path));
    let Ok(bytes) = bytes else {
        return FALLBACK_COLOR;
    };
    let Ok(img) = image::load_from_memory(&bytes) else {
        return FALLBACK_COLOR;
    };
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let cols = atlas[0].max(1) as u32;
    let rows = atlas[1].max(1) as u32;
    let cell_w = (width / cols).max(1);
    let cell_h = (height / rows).max(1);
    let x0 = (frame[0].max(0) as u32).min(cols - 1) * cell_w;
    let y0 = (frame[1].max(0) as u32).min(rows - 1) * cell_h;
    let x1 = (x0 + cell_w).min(width);
    let y1 = (y0 + cell_h).min(height);

    let mut counts: HashMap<[u8; 3], u32> = HashMap::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let pixel = rgba.get_pixel(x, y);
            if pixel[3] < 128 {
                continue;
            }
            *counts.entry([pixel[0], pixel[1], pixel[2]]).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .unwrap_or(FALLBACK_COLOR)
}

impl BurstKind {
    fn sprite_path(self, color: [u8; 3]) -> String {
        let prefix = match self {
            Self::Place => "place",
            Self::Remove => "remove",
        };
        format!(
            "fx/{prefix}/{:02x}{:02x}{:02x}",
            color[0], color[1], color[2]
        )
    }

    fn count(self) -> usize {
        match self {
            Self::Place => 12,
            Self::Remove => 14,
        }
    }

    fn size(self) -> f32 {
        match self {
            Self::Place => 0.12,
            Self::Remove => 0.16,
        }
    }

    fn base_alpha(self) -> f32 {
        match self {
            Self::Place => 0.95,
            Self::Remove => 0.8,
        }
    }

    fn gravity(self) -> f32 {
        match self {
            Self::Place => -1.5,
            Self::Remove => -2.0,
        }
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    dead_entities: Vec<Entity>,
    created_keys: HashSet<u64>,
    seed: u64,
    cache_clear_count: u64,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            dead_entities: Vec::new(),
            created_keys: HashSet::new(),
            seed: 0x9E37_79B9_7F4A_7C15,
            cache_clear_count: 0,
        }
    }

    pub fn burst(&mut self, ecs: &mut EcsAdapter, event: PlacementFxEvent) {
        while self.particles.len() + event.kind.count() > MAX_ACTIVE_PARTICLES {
            let old = self.particles.remove(0);
            if let Some(entity) = old.entity {
                ecs.delete_entity(entity);
            }
        }
        let path = event.kind.sprite_path(event.color);
        for _ in 0..event.kind.count() {
            let (vx, vy, ttl) = match event.kind {
                BurstKind::Place => (
                    (rand01(&mut self.seed) - 0.5) * 2.4,
                    0.8 + rand01(&mut self.seed) * 1.2,
                    0.35 + rand01(&mut self.seed) * 0.25,
                ),
                BurstKind::Remove => (
                    (rand01(&mut self.seed) - 0.5) * 3.2,
                    -0.3 + rand01(&mut self.seed) * 1.1,
                    0.4 + rand01(&mut self.seed) * 0.3,
                ),
            };
            let jitter_x = (rand01(&mut self.seed) - 0.5) * event.width.max(1) as f32 * 0.8;
            let jitter_y = (rand01(&mut self.seed) - 0.5) * event.height.max(1) as f32 * 0.8;
            self.particles.push(Particle {
                kind: event.kind,
                color: event.color,
                path: path.clone(),
                x: event.x + jitter_x,
                y: event.y + jitter_y,
                vx,
                vy,
                age: 0.0,
                ttl,
                entity: None,
            });
        }
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        let mut i = 0;
        while i < self.particles.len() {
            let particle = &mut self.particles[i];
            particle.age += dt;
            if particle.age >= particle.ttl {
                if let Some(entity) = particle.entity.take() {
                    self.dead_entities.push(entity);
                }
                self.particles.swap_remove(i);
                continue;
            }
            let gravity = particle.kind.gravity();
            particle.vy += gravity * dt;
            particle.x += particle.vx * dt;
            particle.y += particle.vy * dt;
            i += 1;
        }
    }

    pub fn sync(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.cache_clear_count != ecs.clear_count {
            self.particles.clear();
            self.dead_entities.clear();
            self.created_keys.clear();
            self.cache_clear_count = ecs.clear_count;
        }
        for entity in self.dead_entities.drain(..) {
            ecs.delete_entity(entity);
        }
        if self.particles.is_empty() {
            return;
        }

        let needed: Vec<(String, BurstKind, [u8; 3])> = self
            .particles
            .iter()
            .map(|p| (p.path.clone(), p.kind, p.color))
            .collect();
        for (path, kind, color) in needed {
            self.ensure_sprite(ecs, device, queue, &path, kind, color);
        }

        for particle in &mut self.particles {
            let alpha = Self::alpha(particle);
            let entity = match particle.entity {
                Some(entity) => entity,
                None => create_sprite(
                    &mut ecs.world,
                    particle.x,
                    particle.y,
                    Z_WEATHER,
                    &particle.path,
                    [0, 0],
                    [1, 1],
                    1.0,
                    alpha,
                ),
            };
            if particle.entity.is_none() {
                particle.entity = Some(entity);
            } else {
                ecs.update_transform_position(entity, particle.x, particle.y);
                ecs.update_sprite_alpha(entity, alpha);
            }
        }
    }

    pub fn reset(&mut self, ecs: &mut EcsAdapter) {
        for particle in &self.particles {
            if let Some(entity) = particle.entity {
                ecs.delete_entity(entity);
            }
        }
        for entity in self.dead_entities.drain(..) {
            ecs.delete_entity(entity);
        }
        for key in self.created_keys.drain() {
            ecs.sprite_cache.remove(&key);
        }
        self.particles.clear();
        self.cache_clear_count = ecs.clear_count;
    }

    fn ensure_sprite(
        &mut self,
        ecs: &mut EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        kind: BurstKind,
        color: [u8; 3],
    ) {
        let key = sprite_cache_key("transparent", path, [0, 0], [1, 1], 1.0);
        self.created_keys.insert(key);
        if ecs.sprite_cache.contains_key(&key) {
            return;
        }
        let rgba = Self::texture_rgba(color);
        let texture = Texture::from_rgba(
            device,
            queue,
            &rgba,
            TEXTURE_SIZE,
            TEXTURE_SIZE,
            "placement_particle",
            wgpu::FilterMode::Nearest,
        );
        let size = kind.size();
        let sprite = Sprite::from_texture(device, &texture, path, size, size);
        ecs.sprite_cache.insert(key, sprite);
    }

    fn texture_rgba(color: [u8; 3]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize);
        for _ in 0..TEXTURE_SIZE * TEXTURE_SIZE {
            rgba.extend_from_slice(&[color[0], color[1], color[2], 255]);
        }
        rgba
    }

    fn alpha(particle: &Particle) -> f32 {
        let t = (particle.age / particle.ttl).clamp(0.0, 1.0);
        particle.kind.base_alpha() * (1.0 - t)
    }

    #[cfg(test)]
    pub fn debug_texture_rgba(color: [u8; 3]) -> Vec<u8> {
        Self::texture_rgba(color)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    #[cfg(test)]
    pub fn snapshot(&self) -> Vec<ParticleSnapshot> {
        self.particles
            .iter()
            .map(|p| ParticleSnapshot {
                kind: p.kind,
                x: p.x,
                y: p.y,
                color: p.color,
                alpha: Self::alpha(p),
            })
            .collect()
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
