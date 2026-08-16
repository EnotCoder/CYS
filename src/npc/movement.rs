// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  movement.rs — движение покупателя по пути
// ========================================================================
//  Содержит логику перемещения покупателя вдоль найденного пути:
//  движение к следующему узлу, анимация ходьбы, поворот по направлению,
//  и финальный шаг к выходу без навигации.
// ========================================================================

use specs::WorldExt;
use crate::data::map::pathfinding::{Node, find_path};
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::scripts::config::BalanceConfig;
use super::ShopperNpc;

pub(crate) fn spawn_path_node(ecs: &EcsAdapter) -> Node {
    let cfg = ecs.world.read_resource::<BalanceConfig>();
    Node::new(cfg.spawn_x, cfg.spawn_y)
}

impl ShopperNpc {
    pub(crate) fn set_texture(&self, ecs: &mut EcsAdapter, texture_path: &str) {
        ecs.update_sprite_texture(self.entity, texture_path);
    }

    pub(crate) fn set_idle(&self, ecs: &mut EcsAdapter) {
        self.set_texture(ecs, self.tex_idle);
    }

    pub(crate) fn set_walk(&self, ecs: &mut EcsAdapter) {
        let tex = if self.walk_frame == 0 { self.tex_walk_1 } else { self.tex_walk_2 };
        self.set_texture(ecs, tex);
    }

    pub(crate) fn start_path(&mut self, ecs: &EcsAdapter, walkable: &std::collections::HashSet<Node>, to: Node) -> bool {
        let from = Node::from_world(self.pos.0, self.pos.1);
        if let Some(path) = find_path(walkable, from, to) {
            self.path = path;
            self.path_index = 0;
            self.walk_timer = 0.0;
            self.walk_frame = 0;
            if to == spawn_path_node(ecs) {
                self.exit_target = Some(self.spawn_world);
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn walk_toward(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.path_index >= self.path.len() {
            return;
        }
        let (anim_interval, speed) = {
            let cfg = ecs.world.read_resource::<BalanceConfig>();
            (cfg.walk_anim_interval, cfg.npc_speed)
        };
        self.walk_timer += dt;
        if self.walk_timer > anim_interval {
            self.walk_timer = 0.0;
            self.walk_frame = 1 - self.walk_frame;
        }
        self.set_walk(ecs);

        let target = self.path[self.path_index];
        let (tx, ty) = target.to_world();
        let (cx, cy) = self.pos;

        let step = speed * dt as f32;
        let dx = tx - cx;
        let dy = ty - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= step || dist < EPSILON {
            self.pos = (tx, ty);
            self.path_index += 1;
        } else {
            self.pos = (cx + dx / dist * step, cy + dy / dist * step);
        }

        let (nx, ny) = self.pos;
        ecs.update_transform_position(self.entity, nx, ny);

        if dx.abs() > 0.01 {
            let facing = if dx > 0.0 { 0.0 } else { std::f32::consts::PI };
            if let Some(rot) = ecs.world.write_storage::<crate::Rotation>().get_mut(self.entity) {
                rot.rotation = [0.0, facing, 0.0];
            }
        }
    }

    /// Финал пути к выходу: движение к точке спавна без навигации.
    /// Возвращает true, когда достигнут выход (нужен деспавн).
    pub(crate) fn walk_to_exit(&mut self, ecs: &mut EcsAdapter, dt: f64) -> bool {
        let Some((ex, ey)) = self.exit_target else {
            return true;
        };
        let (cx, cy) = self.pos;
        let step = ecs.world.read_resource::<BalanceConfig>().npc_speed * dt as f32;
        let dx = ex - cx;
        let dy = ey - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= step || dist < EPSILON {
            return true;
        }
        self.pos = (cx + dx / dist * step, cy + dy / dist * step);
        let (nx, ny) = self.pos;
        ecs.update_transform_position(self.entity, nx, ny);
        false
    }
}
