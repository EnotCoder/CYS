// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use std::collections::HashSet;
use specs::WorldExt;
use crate::data::map::pathfinding::{Node, find_path};
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::ecs::components::{BusyCassas, Money};

mod interactions;
mod movement;

use movement::spawn_path_node;

enum ShopperState {
    GoingToRack,
    GoingToCassa,
    AtCassa,
    GoingToCandies,
    AtCandies,
    GoingToExit,
}

pub struct ShopperNpc {
    pub entity: specs::Entity,
    pos: (f32, f32),
    path: Vec<Node>,
    path_index: usize,
    state: ShopperState,
    state_timer: f64,
    rack_pos: Node,
    cassa_pos: Node,
    candy_pos: Option<Node>,
    candy_taken: bool,
    spawn_world: (f32, f32),
    exit_target: Option<(f32, f32)>,
    exiting: bool,
    food_taken: bool,
    walk_timer: f64,
    walk_frame: i32,
    tex_idle: &'static str,
    tex_walk_1: &'static str,
    tex_walk_2: &'static str,
    alpha: f32,
    despawning: bool,
}

impl ShopperNpc {
    pub fn spawn(ecs: &mut EcsAdapter, walkable: &HashSet<Node>, spawn_pos: Node, rack_pos: Node, cassa_pos: Node, candy_pos: Option<Node>, tex_idle: &'static str, tex_walk_1: &'static str, tex_walk_2: &'static str) -> Option<Self> {
        let path = find_path(walkable, spawn_pos, rack_pos)?;
        let (sx, sy) = spawn_pos.to_world();
        let sy = sy + 0.5;
        let entity = crate::ecs::factory::create_sprite(
            &mut ecs.world, sx, sy, Z_NPC,
            tex_idle, [0, 0], [1, 1], NPC_SCALE, 0.0,
        );
        ecs.world.write_storage::<crate::Rotation>().insert(entity, crate::Rotation { rotation: [0.0; 3] }).ok();
        Some(ShopperNpc {
            entity,
            pos: (sx, sy),
            path,
            path_index: 1,
            state: ShopperState::GoingToRack,
            state_timer: 0.0,
            rack_pos,
            cassa_pos,
            candy_pos,
            candy_taken: false,
            spawn_world: (sx, sy),
            exit_target: None,
            exiting: false,
            food_taken: false,
            walk_timer: 0.0,
            walk_frame: 0,
            tex_idle,
            tex_walk_1,
            tex_walk_2,
            alpha: 0.0,
            despawning: false,
        })
    }

    pub fn set_exiting(&mut self, val: bool) {
        self.exiting = val;
    }

    pub fn has_taken_food(&self) -> bool {
        self.food_taken
    }

    // ================================================================
    //  API для Lua-скриптов (src/scripts/npc.rs)
    // ================================================================

    pub fn state_int(&self) -> i32 {
        match self.state {
            ShopperState::GoingToRack => 1,
            ShopperState::GoingToCassa => 2,
            ShopperState::AtCassa => 3,
            ShopperState::GoingToCandies => 4,
            ShopperState::AtCandies => 5,
            ShopperState::GoingToExit => 6,
        }
    }

    pub fn set_state_int(&mut self, s: i32) {
        self.state = match s {
            2 => ShopperState::GoingToCassa,
            3 => ShopperState::AtCassa,
            4 => ShopperState::GoingToCandies,
            5 => ShopperState::AtCandies,
            6 => ShopperState::GoingToExit,
            _ => ShopperState::GoingToRack,
        };
    }

    pub fn path_done(&self) -> bool {
        self.path_index >= self.path.len()
    }

    pub fn state_timer(&self) -> f64 {
        self.state_timer
    }

    pub fn set_state_timer(&mut self, t: f64) {
        self.state_timer = t;
    }

    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }

    pub fn rack_pos(&self) -> Node {
        self.rack_pos
    }

    pub fn cassa_pos(&self) -> Node {
        self.cassa_pos
    }

    pub fn candy_pos(&self) -> Option<Node> {
        self.candy_pos
    }

    pub fn is_food_taken(&self) -> bool {
        self.food_taken
    }

    pub fn is_exiting(&self) -> bool {
        self.exiting
    }

    /// Запускает деспавн (фейд-аут). Вызывается из Lua.
    pub(crate) fn request_despawn(&mut self) {
        self.despawning = true;
    }

    pub fn update(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>, script: Option<&crate::scripts::npc::NpcScript>) -> bool {
        let fade = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().npc_fade_speed;
        if self.despawning {
            self.alpha = (self.alpha - dt as f32 * fade).max(0.0);
            ecs.update_sprite_alpha(self.entity, self.alpha);
            if self.alpha <= 0.0 {
                return true;
            }
            return false;
        }

        if self.alpha < 1.0 {
            self.alpha = (self.alpha + dt as f32 * fade).min(1.0);
            ecs.update_sprite_alpha(self.entity, self.alpha);
        }

        // Пытаемся использовать Lua-скрипт. При ошибке — fallback на Rust-автомат.
        if let Some(script) = script {
            if script.update(self, ecs, dt, walkable).is_err() {
                self.update_fallback(ecs, dt, walkable);
            }
            // Фейд-аут уже обрабатывается в начале update на следующих кадрах.
            return false;
        }

        self.update_fallback(ecs, dt, walkable)
    }

    /// Оригинальный Rust-автомат (fallback, если не загружен/упал Lua-скрипт).
    fn update_fallback(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>) -> bool {

        // Если касса удалена — переключиться на другую или уйти
        if !matches!(self.state, ShopperState::GoingToExit) && !self.cassa_exists(ecs) {
            if let Some(cassa) = Self::find_any_cassa(ecs) {
                if self.food_taken {
                    self.reroute_to_cassa(ecs, walkable, cassa);
                } else {
                    self.cassa_pos = cassa;
                }
            } else {
                // Касс нет — уходим
                if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                    self.state = ShopperState::GoingToExit;
                    return false;
                }
                self.despawning = true;
                return false;
            }
        }

        // Если стеллаж удалён или пуст — деспавн
        if !self.food_taken && matches!(self.state, ShopperState::GoingToRack) && !self.rack_exists(ecs) {
            self.despawning = true;
            return false;
        }

        // Если active=false и ещё не взял еду — уходим
        if self.exiting && !self.food_taken && matches!(self.state, ShopperState::GoingToRack) {
            if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                self.state = ShopperState::GoingToExit;
            }
        }
        // Если active=true и шёл на выход без покупки — возвращаемся к rack
        if !self.exiting && !self.food_taken && matches!(self.state, ShopperState::GoingToExit) {
            if self.start_path(ecs, walkable, self.rack_pos) {
                self.state = ShopperState::GoingToRack;
            }
        }

        match self.state {
            ShopperState::GoingToRack => {
                if self.path_index >= self.path.len() {
                    self.set_idle(ecs);
                    if self.exiting {
                        self.despawning = true;
                        return false;
                    }
                    if !self.cassa_exists(ecs) {
                        ecs.world.write_resource::<BusyCassas>().0.remove(&(self.cassa_pos.x, self.cassa_pos.y));
                        if let Some(cassa) = Self::find_any_cassa(ecs) {
                            self.cassa_pos = cassa;
                        } else {
                            if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                                self.state = ShopperState::GoingToExit;
                            }
                            return false;
                        }
                    }
                    let cp = (self.cassa_pos.x, self.cassa_pos.y);
                    if ecs.world.read_resource::<BusyCassas>().0.contains(&cp) {
                        return false;
                    }
                    if !self.take_food(ecs) {
                        self.despawning = true;
                        return false;
                    }
                    ecs.world.write_resource::<BusyCassas>().0.insert(cp);
                    if self.start_path(ecs, walkable, self.cassa_pos) {
                        self.state = ShopperState::GoingToCassa;
                    }
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::GoingToCassa => {
                if self.path_index >= self.path.len() {
                    self.set_idle(ecs);
                    self.state = ShopperState::AtCassa;
                    self.state_timer = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().cassa_wait_secs;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::AtCassa => {
                self.state_timer -= dt;
                self.set_idle(ecs);
                if self.state_timer <= 0.0 {
                    ecs.world.write_resource::<BusyCassas>().0.remove(&(self.cassa_pos.x, self.cassa_pos.y));
                    let cfg = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>();
                    ecs.world.write_resource::<Money>().0 += cfg.money_at_cassa;
                    crate::audio::play("cash");
                    // Шанс зайти за конфетами (config.candy_chance)
                    let want_candy = self.candy_pos.is_some()
                        && (cfg.candy_chance as f64) > 0.0
                        && std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() % 100 < (cfg.candy_chance * 100.0) as u32
                        && self.candy_exists(ecs);
                    if want_candy {
                        if self.start_path(ecs, walkable, self.candy_pos.unwrap()) {
                            self.state = ShopperState::GoingToCandies;
                        } else {
                            if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                                self.state = ShopperState::GoingToExit;
                            }
                        }
                    } else {
                        if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                            self.state = ShopperState::GoingToExit;
                        }
                    }
                }
            }

            ShopperState::GoingToCandies => {
                // If candy was deleted or ran out, skip to exit
                if !self.candy_exists(ecs) {
                    if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                        self.state = ShopperState::GoingToExit;
                    }
                    return false;
                }
                if self.path_index >= self.path.len() {
                    self.set_idle(ecs);
                    self.take_candy(ecs);
                    self.state = ShopperState::AtCandies;
                    self.state_timer = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().candy_wait_secs;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::AtCandies => {
                self.state_timer -= dt;
                self.set_idle(ecs);
                if self.state_timer <= 0.0 {
                    ecs.world.write_resource::<Money>().0 += ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().money_at_candy;
                    crate::audio::play("cash");
                    if self.start_path(ecs, walkable, spawn_path_node(ecs)) {
                        self.state = ShopperState::GoingToExit;
                    }
                }
            }

            ShopperState::GoingToExit => {
                if self.path_index >= self.path.len() {
                    if let Some((ex, ey)) = self.exit_target {
                        let (cx, cy) = self.pos;
                        let step = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().npc_speed * dt as f32;
                        let dx = ex - cx;
                        let dy = ey - cy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist <= step || dist < EPSILON {
                            self.despawning = true;
                            return false;
                        }
                        self.pos = (cx + dx / dist * step, cy + dy / dist * step);
                        let (nx, ny) = self.pos;
                        ecs.update_transform_position(self.entity, nx, ny);
                        return false;
                    }
                    self.despawning = true;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }
        }
        false
    }
}
