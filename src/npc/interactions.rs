// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  interactions.rs — взаимодействие покупателя с миром
// ========================================================================
//  Содержит методы для проверки и изменения состояния объектов:
//  снятие еды со стеллажа, взятие конфет, проверка существования
//  кассы/стеллажа/конфет, поиск любой кассы и маршрутизация к ней.
// ========================================================================

use specs::WorldExt;
use specs::Join;
use crate::data::map::pathfinding::Node;
use crate::EcsAdapter;
use crate::ecs::components::{FoodStorage, ObjectTag, Transform, BusyCassas};
use super::{ShopperNpc, ShopperState};

impl ShopperNpc {
    pub(crate) fn take_food(&mut self, ecs: &mut EcsAdapter) -> bool {
        let rn = self.rack_pos;
        let taken = {
            let tags = ecs.world.read_storage::<ObjectTag>();
            let transforms = ecs.world.read_storage::<Transform>();
            let mut foods = ecs.world.write_storage::<FoodStorage>();
            let mut taken = false;
            for (tag, transform, food) in (&tags, &transforms, &mut foods).join() {
                if tag.name == "rack"
                    && transform.position[0] as i32 == rn.x
                    && (transform.position[1] as i32 == rn.y || transform.position[1] as i32 == rn.y - 1)
                    && food.food_count > 0
                {
                    food.food_count -= 1;
                    taken = true;
                    break;
                }
            }
            taken
        };
        if taken {
            ecs.update_object_textures();
            self.food_taken = true;
            crate::audio::play("pickup");
        }
        taken
    }

    pub fn despawn(&mut self, ecs: &mut EcsAdapter) {
        ecs.delete_entity(self.entity);
    }

    pub(crate) fn candy_exists(&self, ecs: &EcsAdapter) -> bool {
        let Some(cp) = self.candy_pos else { return false };
        let tags = ecs.world.read_storage::<ObjectTag>();
        let transforms = ecs.world.read_storage::<Transform>();
        let foods = ecs.world.read_storage::<FoodStorage>();
        for (tag, transform, food) in (&tags, &transforms, &foods).join() {
            if tag.name == "candies"
                && transform.position[0] as i32 == cp.x
                && transform.position[1] as i32 == cp.y
                && food.food_count > 0
            {
                return true;
            }
        }
        false
    }

    pub(crate) fn take_candy(&mut self, ecs: &mut EcsAdapter) -> bool {
        let Some(cp) = self.candy_pos else { return false };
        let taken = {
            let tags = ecs.world.read_storage::<ObjectTag>();
            let transforms = ecs.world.read_storage::<Transform>();
            let mut foods = ecs.world.write_storage::<FoodStorage>();
            let mut taken = false;
            for (tag, transform, food) in (&tags, &transforms, &mut foods).join() {
                if tag.name == "candies"
                    && transform.position[0] as i32 == cp.x
                    && transform.position[1] as i32 == cp.y
                    && food.food_count > 0
                {
                    food.food_count -= 1;
                    taken = true;
                    break;
                }
            }
            taken
        };
        if taken {
            ecs.update_object_textures();
            self.candy_taken = true;
            crate::audio::play("candy");
        }
        taken
    }

    pub(crate) fn cassa_exists(&self, ecs: &EcsAdapter) -> bool {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let transforms = ecs.world.read_storage::<Transform>();
        for (tag, transform) in (&tags, &transforms).join() {
            if tag.name == "cassa"
                && transform.position[0] as i32 == self.cassa_pos.x
                && transform.position[1] as i32 == self.cassa_pos.y
            {
                return true;
            }
        }
        false
    }

    pub(crate) fn rack_exists(&self, ecs: &EcsAdapter) -> bool {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let transforms = ecs.world.read_storage::<Transform>();
        let foods = ecs.world.read_storage::<FoodStorage>();
        for (tag, transform, food) in (&tags, &transforms, &foods).join() {
            if tag.name == "rack"
                && transform.position[0] as i32 == self.rack_pos.x
                && (transform.position[1] as i32 == self.rack_pos.y || transform.position[1] as i32 == self.rack_pos.y - 1)
                && food.food_count > 0
            {
                return true;
            }
        }
        false
    }

    pub(crate) fn find_any_cassa(ecs: &EcsAdapter) -> Option<Node> {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let transforms = ecs.world.read_storage::<Transform>();
        for (tag, transform) in (&tags, &transforms).join() {
            if tag.name == "cassa" {
                return Some(Node::new(transform.position[0] as i32, transform.position[1] as i32));
            }
        }
        None
    }

    pub(crate) fn reroute_to_cassa(&mut self, ecs: &mut EcsAdapter, walkable: &std::collections::HashSet<Node>, cp: Node) {
        ecs.world.write_resource::<BusyCassas>().0.remove(&(self.cassa_pos.x, self.cassa_pos.y));
        ecs.world.write_resource::<BusyCassas>().0.insert((cp.x, cp.y));
        self.cassa_pos = cp;
        if self.start_path(ecs, walkable, cp) {
            self.state = ShopperState::GoingToCassa;
        }
    }
}
