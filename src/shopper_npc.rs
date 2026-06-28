use std::collections::HashSet;
use specs::WorldExt;
use specs::Join;
use crate::pathfinding::{Node, find_path};
use crate::constants::*;
use crate::EcsAdapter;
use crate::ecs::components::{FoodStorage, ObjectTag, Transform, CassaBusy};

enum ShopperState {
    GoingToRack,
    GoingToCassa,
    AtCassa,
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
    walk_timer: f64,
    walk_frame: i32,
}

impl ShopperNpc {
    /// Создать покупателя. spawn_pos должен быть проходим.
    pub fn spawn(ecs: &mut EcsAdapter, walkable: &HashSet<Node>, spawn_pos: Node, rack_pos: Node, cassa_pos: Node) -> Option<Self> {
        let path = find_path(walkable, spawn_pos, rack_pos)?;
        let (sx, sy) = spawn_pos.to_world();
        let entity = crate::ecs::factory::create_sprite(
            &mut ecs.world, sx, sy, Z_NPC,
            TEX_PLAYER_IDLE, [0, 0], [1, 1], NPC_SCALE, 1.0,
        );
        ecs.world.write_storage::<crate::Rotation>().insert(entity, crate::Rotation { rotation: [0.0; 3] }).ok();
        Some(ShopperNpc {
            entity,
            pos: (sx, sy),
            path,
            path_index: 0,
            state: ShopperState::GoingToRack,
            state_timer: 0.0,
            rack_pos,
            cassa_pos,
            walk_timer: 0.0,
            walk_frame: 0,
        })
    }

    fn set_texture(&self, ecs: &mut EcsAdapter, texture_path: &str) {
        ecs.update_sprite_texture(self.entity, texture_path);
    }

    fn walk_toward(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.path_index >= self.path.len() {
            return;
        }
        self.walk_timer += dt;
        if self.walk_timer > WALK_ANIM_INTERVAL {
            self.walk_timer = 0.0;
            self.walk_frame = 1 - self.walk_frame;
        }
        let tex = if self.walk_frame == 0 { TEX_PLAYER_WALK_1 } else { TEX_PLAYER_WALK_2 };
        self.set_texture(ecs, tex);

        let target = self.path[self.path_index];
        let (tx, ty) = target.to_world();
        let (cx, cy) = self.pos;

        let step = NPC_SPEED * dt as f32;
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

    /// Уменьшить еду на стойке на 1
    fn take_food(&self, ecs: &mut EcsAdapter) -> bool {
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
        }
        taken
    }

    /// Деспавн — удаление entity
    pub fn despawn(&mut self, ecs: &mut EcsAdapter) {
        ecs.delete_entity(self.entity);
    }

    /// Вернёт true, когда пора деспавниться
    pub fn update(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>) -> bool {
        match self.state {
            ShopperState::GoingToRack => {
                if self.path_index >= self.path.len() {
                    self.set_texture(ecs, TEX_PLAYER_IDLE);
                    if ecs.world.read_resource::<CassaBusy>().0 {
                        return false; // касса занята — ждём, еду НЕ берем
                    }
                    if !self.take_food(ecs) {
                        return true; // еды нет — уходим
                    }
                    ecs.world.write_resource::<CassaBusy>().0 = true;
                    let from = Node::from_world(self.pos.0, self.pos.1);
                    if let Some(path) = find_path(walkable, from, self.cassa_pos) {
                        self.path = path;
                        self.path_index = 0;
                        self.state = ShopperState::GoingToCassa;
                        self.walk_timer = 0.0;
                        self.walk_frame = 0;
                    }
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::GoingToCassa => {
                if self.path_index >= self.path.len() {
                    self.set_texture(ecs, TEX_PLAYER_IDLE);
                    self.state = ShopperState::AtCassa;
                    self.state_timer = CASSA_WAIT_SECS;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::AtCassa => {
                self.state_timer -= dt;
                self.set_texture(ecs, TEX_PLAYER_IDLE);
                if self.state_timer <= 0.0 {
                    ecs.world.write_resource::<CassaBusy>().0 = false;
                    return true; // деспавн
                }
            }
        }
        false
    }
}
