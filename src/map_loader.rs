// ========================================================================
//  Загрузка карты из map.txt в ECS мир
// ========================================================================

use std::fs::File;
use std::io::{BufRead, BufReader};
use specs::{WorldExt, Builder};
use crate::ecs::{Transform, SpriteComponent, EcsAdapter};
use crate::constants::{WORLD_OFFSET_X, WORLD_OFFSET_Y, Z_MAP};

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

            ecs.world
                .create_entity()
                .with(Transform {
                    position: [x, y, Z_MAP],
                })
                .with(SpriteComponent {
                    texture_path: tex_path.to_string(),
                    texture_frame: tex_pos,
                    texture_count: tex_count,
                    scale: 1.0,
                    alpha: 1.0,
                })
                .build();
        }
    }
}

fn token_to_texture(token: &str) -> (&str, [i32; 2], [i32; 2]) {
    match token {
        "." => ("tex/grass.png", [0, 0], [4, 4]),
        "+" => ("tex/grass.png", [3, 1], [4, 4]),
        "@" => ("tex/grass.png", [0, 2], [4, 4]),
        "*" => ("tex/grass.png", [2, 2], [4, 4]),
        "m" => ("tex/grass.png", [3, 2], [4, 4]),
        "~" => ("tex/grass.png", [1, 2], [4, 4]),
        "l" => ("tex/grass.png", [0, 3], [4, 4]),
        "$" => ("tex/grass.png", [1, 3], [4, 4]),
        //shadow
        "1" => ("tex/grass.png", [0, 1], [4, 4]),
        "2" => ("tex/grass.png", [1, 1], [4, 4]),
        "3" => ("tex/grass.png", [2, 1], [4, 4]),
        "4" => ("tex/grass.png", [1, 0], [4, 4]),
        "5" => ("tex/grass.png", [2, 0], [4, 4]),
        "6" => ("tex/grass.png", [3, 0], [4, 4]),
        "0" => ("tex/floor.png", [0, 0], [2, 2]),
        "=" => ("tex/wall.png", [0, 0], [5, 5]),
        "-" => ("tex/wall.png", [0, 1], [5, 5]),
        "^" => ("tex/wall.png", [1, 0], [5, 5]),
        "&" => ("tex/wall.png", [1, 1], [5, 5]),
        "/" => ("tex/wall.png", [0, 2], [5, 5]),
        "|" => ("tex/wall.png", [1, 2], [5, 5]),
        "(" => ("tex/wall.png", [0, 3], [5, 5]),
        "{" => ("tex/wall.png", [0, 4], [5, 5]),
        ")" => ("tex/wall.png", [1, 3], [5, 5]),
        "}" => ("tex/wall.png", [1, 4], [5, 5]),
        //window
        "[" => ("tex/wall.png", [2, 0], [5, 5]),
        "]" => ("tex/wall.png", [4, 0], [5, 5]),
        ":" => ("tex/wall.png", [2, 1], [5, 5]),
        ";" => ("tex/wall.png", [4, 1], [5, 5]),
        "o" => ("tex/wall.png", [3, 0], [5, 5]),
        "%" => ("tex/wall.png", [3, 1], [5, 5]),
        //default
        _    => ("tex/floor.png", [0, 0], [2, 2]),
    }
}
