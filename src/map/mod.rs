pub mod pathfinding;

// ========================================================================
//  Загрузка карты из map.txt в ECS мир
// ========================================================================

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::ecs::EcsAdapter;
use crate::constants::{WORLD_OFFSET_X, WORLD_OFFSET_Y, Z_MAP};
use crate::map::pathfinding::Node;

/// Загружает map.txt и создаёт для каждой клетки ECS-сущность
pub fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open(crate::constants::MAP_FILE).expect("map.txt not found!");
    let reader = BufReader::new(file);

    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        for (i, token) in parts.iter().enumerate() {
            let (tex_path, tex_pos, tex_count) = token_to_texture(token);

            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;
            let grid_x = (x + 0.5).floor() as i32;
            let grid_y = (y + 0.5).floor() as i32;

            if *token == "=" || *token == "-" {
                ecs.wall_positions.insert((grid_x, grid_y));
            } else if *token == "0" {
                ecs.floor_positions.insert((grid_x, grid_y));
            } else if *token == "." {
                ecs.outdoor_positions.insert((grid_x, grid_y));
            }
            if *token == "." {
                ecs.flower_positions.insert((grid_x, grid_y));
            }

            crate::ecs::factory::create_sprite(
                &mut ecs.world, x, y, Z_MAP,
                tex_path, tex_pos, tex_count, 1.0, 1.0,
            );
        }
    }
}

/// Загружает проходимые клетки из map.txt (для NPC pathfinding)
pub fn load_walkable_cells() -> HashSet<Node> {
    let src = include_str!("../../map.txt");
    let mut cells = HashSet::new();
    for (j, line) in src.lines().enumerate() {
        for (i, token) in line.split_whitespace().enumerate() {
            if matches!(token, "@" | "!" | "." | "~" | "0" | "*" | "m" | "f" | "l" | "1" | "2" | "3" | "4" | "5" | "6") {
                let wx = i as f32 + WORLD_OFFSET_X;
                let wy = -(j as f32) + WORLD_OFFSET_Y;
                cells.insert(Node::from_world(wx, wy));
            }
        }
    }
    // Дверные тайлы магазина — стены визуально, но проходимы для NPC
    cells.insert(Node::new(0, -5));
    cells.insert(Node::new(0, -6));
    cells
}

/// Находит точку спавна покупателей — самая нижняя клетка `0` (пол магазина),
/// ближайшая к центру по X.
pub fn shopper_spawn_point() -> Node {
    let src = include_str!("../../map.txt");
    let mut best: Option<Node> = None;
    for (j, line) in src.lines().enumerate() {
        for (i, token) in line.split_whitespace().enumerate() {
            if token == "0" {
                let node = Node::new(i as i32 + WORLD_OFFSET_X as i32, -(j as i32) + WORLD_OFFSET_Y as i32);
                let better = match best {
                    None => true,
                    Some(cur) => node.y < cur.y || (node.y == cur.y && node.x.abs() < cur.x.abs()),
                };
                if better {
                    best = Some(node);
                }
            }
        }
    }
    best.unwrap_or(Node::new(0, -4))
}

fn token_to_texture(token: &str) -> (&str, [i32; 2], [i32; 2]) {
    match token {
        "." => ("tex/map/grass.png", [0, 0], [4, 4]),
        "@" => ("tex/map/grass.png", [0, 2], [4, 4]),
        "*" => ("tex/map/grass.png", [2, 2], [4, 4]),
        "m" => ("tex/map/grass.png", [3, 2], [4, 4]),
        "f" => ("tex/map/grass.png", [2, 3], [4, 4]),
        "~" => ("tex/map/grass.png", [1, 2], [4, 4]),
        "l" => ("tex/map/grass.png", [0, 3], [4, 4]),
        //shadow
        "1" => ("tex/map/grass.png", [0, 1], [4, 4]),
        "2" => ("tex/map/grass.png", [1, 1], [4, 4]),
        "3" => ("tex/map/grass.png", [2, 1], [4, 4]),
        "4" => ("tex/map/grass.png", [1, 0], [4, 4]),
        "5" => ("tex/map/grass.png", [2, 0], [4, 4]),
        "6" => ("tex/map/grass.png", [3, 0], [4, 4]),
        "0" => ("tex/map/floor.png", [0, 0], [2, 2]),
        "=" => ("tex/map/wall.png", [0, 0], [5, 5]),
        "-" => ("tex/map/wall.png", [0, 1], [5, 5]),
        "^" => ("tex/map/wall.png", [1, 0], [5, 5]),
        "&" => ("tex/map/wall.png", [1, 1], [5, 5]),
        "/" => ("tex/map/wall.png", [0, 2], [5, 5]),
        "|" => ("tex/map/wall.png", [1, 2], [5, 5]),
        "(" => ("tex/map/wall.png", [0, 3], [5, 5]),
        "{" => ("tex/map/wall.png", [0, 4], [5, 5]),
        ")" => ("tex/map/wall.png", [1, 3], [5, 5]),
        "}" => ("tex/map/wall.png", [1, 4], [5, 5]),
        //window
        "[" => ("tex/map/wall.png", [2, 0], [5, 5]),
        "]" => ("tex/map/wall.png", [4, 0], [5, 5]),
        ":" => ("tex/map/wall.png", [2, 1], [5, 5]),
        ";" => ("tex/map/wall.png", [4, 1], [5, 5]),
        "o" => ("tex/map/wall.png", [3, 0], [5, 5]),
        "%" => ("tex/map/wall.png", [3, 1], [5, 5]),
        //default
        _    => ("tex/map/floor.png", [0, 0], [2, 2]),
    }
}
