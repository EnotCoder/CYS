use std::collections::HashSet;
use specs::WorldExt;
use specs::Join;
use crate::map::pathfinding::{Node, find_path};
use crate::constants::*;
use crate::EcsAdapter;
use crate::ecs::components::{FoodStorage, ObjectTag, Transform, BusyCassas, Money};

fn spawn_path_node() -> Node {
    Node::new(0, -3)
}

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

    fn set_texture(&self, ecs: &mut EcsAdapter, texture_path: &str) {
        ecs.update_sprite_texture(self.entity, texture_path);
    }

    fn set_idle(&self, ecs: &mut EcsAdapter) {
        self.set_texture(ecs, self.tex_idle);
    }

    fn set_walk(&self, ecs: &mut EcsAdapter) {
        let tex = if self.walk_frame == 0 { self.tex_walk_1 } else { self.tex_walk_2 };
        self.set_texture(ecs, tex);
    }

    fn start_path(&mut self, walkable: &HashSet<Node>, to: Node) -> bool {
        let from = Node::from_world(self.pos.0, self.pos.1);
        if let Some(path) = find_path(walkable, from, to) {
            self.path = path;
            self.path_index = 0;
            self.walk_timer = 0.0;
            self.walk_frame = 0;
            if to == spawn_path_node() {
                self.exit_target = Some(self.spawn_world);
            }
            true
        } else {
            false
        }
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
        self.set_walk(ecs);

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

    fn take_food(&mut self, ecs: &mut EcsAdapter) -> bool {
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
        }
        taken
    }

    pub fn despawn(&mut self, ecs: &mut EcsAdapter) {
        ecs.delete_entity(self.entity);
    }

    fn candy_exists(&self, ecs: &EcsAdapter) -> bool {
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

    fn take_candy(&mut self, ecs: &mut EcsAdapter) -> bool {
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
        }
        taken
    }

    fn cassa_exists(&self, ecs: &EcsAdapter) -> bool {
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

    fn rack_exists(&self, ecs: &EcsAdapter) -> bool {
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

    fn find_any_cassa(ecs: &EcsAdapter) -> Option<Node> {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let transforms = ecs.world.read_storage::<Transform>();
        for (tag, transform) in (&tags, &transforms).join() {
            if tag.name == "cassa" {
                return Some(Node::new(transform.position[0] as i32, transform.position[1] as i32));
            }
        }
        None
    }

    fn reroute_to_cassa(&mut self, ecs: &mut EcsAdapter, walkable: &HashSet<Node>, cp: Node) {
        ecs.world.write_resource::<BusyCassas>().0.remove(&(self.cassa_pos.x, self.cassa_pos.y));
        ecs.world.write_resource::<BusyCassas>().0.insert((cp.x, cp.y));
        self.cassa_pos = cp;
        if self.start_path(walkable, cp) {
            self.state = ShopperState::GoingToCassa;
        }
    }

    pub fn update(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>) -> bool {
        if self.despawning {
            self.alpha = (self.alpha - dt as f32 * 2.0).max(0.0);
            ecs.update_sprite_alpha(self.entity, self.alpha);
            if self.alpha <= 0.0 {
                return true;
            }
            return false;
        }

        if self.alpha < 1.0 {
            self.alpha = (self.alpha + dt as f32 * 2.0).min(1.0);
            ecs.update_sprite_alpha(self.entity, self.alpha);
        }

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
                if self.start_path(walkable, spawn_path_node()) {
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
            if self.start_path(walkable, spawn_path_node()) {
                self.state = ShopperState::GoingToExit;
            }
        }
        // Если active=true и шёл на выход без покупки — возвращаемся к rack
        if !self.exiting && !self.food_taken && matches!(self.state, ShopperState::GoingToExit) {
            if self.start_path(walkable, self.rack_pos) {
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
                            if self.start_path(walkable, spawn_path_node()) {
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
                    if self.start_path(walkable, self.cassa_pos) {
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
                    self.state_timer = CASSA_WAIT_SECS;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::AtCassa => {
                self.state_timer -= dt;
                self.set_idle(ecs);
                if self.state_timer <= 0.0 {
                    ecs.world.write_resource::<BusyCassas>().0.remove(&(self.cassa_pos.x, self.cassa_pos.y));
                    ecs.world.write_resource::<Money>().0 += 5;
                    // 20% chance to visit candies
                    let want_candy = self.candy_pos.is_some()
                        && std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() % 5 == 0
                        && self.candy_exists(ecs);
                    if want_candy {
                        if self.start_path(walkable, self.candy_pos.unwrap()) {
                            self.state = ShopperState::GoingToCandies;
                        } else {
                            if self.start_path(walkable, spawn_path_node()) {
                                self.state = ShopperState::GoingToExit;
                            }
                        }
                    } else {
                        if self.start_path(walkable, spawn_path_node()) {
                            self.state = ShopperState::GoingToExit;
                        }
                    }
                }
            }

            ShopperState::GoingToCandies => {
                // If candy was deleted or ran out, skip to exit
                if !self.candy_exists(ecs) {
                    if self.start_path(walkable, spawn_path_node()) {
                        self.state = ShopperState::GoingToExit;
                    }
                    return false;
                }
                if self.path_index >= self.path.len() {
                    self.set_idle(ecs);
                    self.take_candy(ecs);
                    self.state = ShopperState::AtCandies;
                    self.state_timer = 3.0;
                    return false;
                }
                self.walk_toward(ecs, dt);
            }

            ShopperState::AtCandies => {
                self.state_timer -= dt;
                self.set_idle(ecs);
                if self.state_timer <= 0.0 {
                    ecs.world.write_resource::<Money>().0 += 1;
                    if self.start_path(walkable, spawn_path_node()) {
                        self.state = ShopperState::GoingToExit;
                    }
                }
            }

            ShopperState::GoingToExit => {
                if self.path_index >= self.path.len() {
                    if let Some((ex, ey)) = self.exit_target {
                        let (cx, cy) = self.pos;
                        let step = NPC_SPEED * dt as f32;
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
