use std::collections::HashSet;
use specs::WorldExt;
use crate::pathfinding::{Node, find_path};
use crate::constants::*;
use crate::EcsAdapter;

// ========================================================================
//  Маршруты патрулирования NPC
// ========================================================================

fn patrol_routes() -> Vec<Vec<Node>> {
    vec![
        vec![
            Node::new(13, 11),  Node::new(13, -5),
            Node::new(13, -11), Node::new(-14, -11),
            Node::new(-15, 11),
        ],
        vec![
            Node::new(-15, 11),  Node::new(-14, -11),
            Node::new(13, -11),  Node::new(13, -5),
            Node::new(13, 11),
        ],
        vec![
            Node::new(-15, 11),  Node::new(-14, -11),
            Node::new(13, -11),  Node::new(13, -8),
            Node::new(-14, -8),  Node::new(-15, 11),
        ],
        vec![
            Node::new(9, 11),   Node::new(9, -5),
            Node::new(10, -5),  Node::new(10, -11),
            Node::new(3, -11),  Node::new(3, -9),
            Node::new(13, -9),  Node::new(13, 11),
        ],
        vec![
            Node::new(13, -5),  Node::new(-15, -5),
            Node::new(-14, -8), Node::new(13, -8),
            Node::new(13, -11), Node::new(-14, -11),
            Node::new(-15, 11),
        ],
    ]
}

// ========================================================================
//  NPC — сущность, перемещающаяся по карте
// ========================================================================

pub struct Npc {
    pub entity: specs::Entity,
    pos: (f32, f32),
    path: Vec<Node>,
    path_index: usize,
    patrol_route: Vec<Node>,
    patrol_index: usize,
    pause: f64,
    walk_timer: f64,
    walk_frame: i32,
}

impl Npc {
    pub fn new(ecs: &mut EcsAdapter, route: &[Node], start_idx: usize) -> Self {
        let start = route[start_idx];
        let (sx, sy) = start.to_world();
        let entity = crate::ecs::factory::create_sprite(
            &mut ecs.world, sx, sy, Z_NPC,
            TEX_PLAYER_IDLE, [0, 0], [1, 1], NPC_SCALE, 1.0,
        );
        ecs.world.write_storage::<crate::Rotation>().insert(entity, crate::Rotation { rotation: [0.0; 3] }).ok();
        Npc {
            entity,
            pos: (sx, sy),
            path: Vec::new(),
            path_index: 0,
            patrol_route: route.to_vec(),
            patrol_index: start_idx,
            pause: 0.0,
            walk_timer: 0.0,
            walk_frame: 0,
        }
    }

    pub fn advance(&mut self, walkable: &HashSet<Node>) {
        loop {
            let (cx, cy) = self.pos;
            let start_node = Node::from_world(cx, cy);
            self.patrol_index = (self.patrol_index + 1) % self.patrol_route.len();
            let goal_node = self.patrol_route[self.patrol_index];
            if start_node == goal_node {
                continue;
            }
            if let Some(path) = find_path(walkable, start_node, goal_node) {
                self.path = path;
                self.path_index = 0;
            }
            break;
        }
    }

    fn set_texture(&self, ecs: &mut EcsAdapter, texture_path: &str) {
        ecs.update_sprite_texture(self.entity, texture_path);
    }

    pub fn update(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>) {
        if self.pause > 0.0 {
            self.pause -= dt;
            self.set_texture(ecs, TEX_PLAYER_IDLE);
            return;
        }

        if self.path_index >= self.path.len() {
            self.set_texture(ecs, TEX_PLAYER_IDLE);
            self.pause = NPC_PAUSE_DURATION;
            self.advance(walkable);
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
}

// ========================================================================
//  Создание и обновление всех NPC
// ========================================================================

pub fn setup_npcs(ecs: &mut EcsAdapter, walkable: &HashSet<Node>) -> Vec<Npc> {
    let routes = patrol_routes();
    let start_indices = [0, 0, 2, 0, 2];
    let mut npcs = Vec::new();
    for (idx, route) in routes.iter().enumerate() {
        let start_idx = start_indices[idx.min(start_indices.len() - 1)] % route.len();
        let mut npc = Npc::new(ecs, route, start_idx);
        npc.advance(walkable);
        npcs.push(npc);
    }
    npcs
}

pub fn move_npcs(npcs: &mut Vec<Npc>, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>) {
    for npc in npcs {
        npc.update(ecs, dt, walkable);
    }
}
