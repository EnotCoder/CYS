pub mod pathfinding;

// ========================================================================
//  Загрузка карты из map.txt в ECS мир
// ========================================================================
//  map.txt описывает уровень строками токенов, разделённых пробелами.
//  Каждый токен кодирует клетку: "=" / "-" — стены, "0" — пол магазина,
//  "." и прочие — трава снаружи, "/" / "|" / "&" — стены, на которые можно
//  ставить предметы, "^" / "[" / "]" и др. — декоративные стены и окна.
//  Здесь же происходит разбор файла и создание ECS-сущностей земли.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead};
use crate::ecs::EcsAdapter;
use crate::constants::{WORLD_OFFSET_X, WORLD_OFFSET_Y, Z_MAP};
use crate::map::pathfinding::Node;

/// Загружает карту из файла и создаёт для каждой клетки ECS-сущность
pub fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open(crate::constants::MAP_FILE).expect("map.txt not found!");
    load_map_from_reader(ecs, file, false);
}

/// Загружает карту подвала из отдельного файла
pub fn load_basement_to_ecs(ecs: &mut EcsAdapter) {
    let file = File::open(crate::constants::BASEMENT_FILE).expect("basement.txt not found!");
    load_map_from_reader(ecs, file, true);
}

/// Читает текстовую карту построчно и превращает каждый токен в спрайт-сущность
fn load_map_from_reader(ecs: &mut EcsAdapter, reader: impl std::io::Read, _is_basement: bool) {
    let reader = std::io::BufReader::new(reader);

    // j — номер строки (ось Y), i — позиция в строке (ось X)
    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let mut grid_row: Vec<String> = Vec::new();
        for (i, token) in parts.iter().enumerate() {
            grid_row.push(token.to_string());
            let (tex_path, tex_pos, tex_count) = token_to_texture(token);

            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;
            let grid_x = (x + 0.5).floor() as i32;
            let grid_y = (y + 0.5).floor() as i32;

            // Сохраняем исходный токен клетки, чтобы уметь восстанавливать уровень
            ecs.original_tokens.insert((grid_x, grid_y), token.to_string());

            // Токены травы/улицы — помечаем клетки как outdoor и пригодные к посадке цветов
            let is_grass = matches!(*token, "." | "@" | "*" | "m" | "f" | "~" | "l" | "1" | "2" | "3" | "4" | "5" | "6");
            if *token == "=" || *token == "-" {
                ecs.wall_positions.insert((grid_x, grid_y));
            } else if *token == "0" {
                ecs.floor_positions.insert((grid_x, grid_y));
            }
            if is_grass {
                ecs.outdoor_positions.insert((grid_x, grid_y));
                ecs.flower_positions.insert((grid_x, grid_y));
            }
            // Стены с полкой сверху: на "/" "|" и "." можно ставить предметы,
            // "&" — только если стена не является нижней частью ряда
            if matches!(*token, "/" | "|" | ".") {
                ecs.floor_placeable_positions.insert((grid_x, grid_y));
            } else if *token == "&" {
                let is_bottom_wall = j > 0 && ecs.map_grid.get(j - 1)
                    .and_then(|row| row.get(i))
                    .map_or(false, |t| t == "0");
                if !is_bottom_wall {
                    ecs.floor_placeable_positions.insert((grid_x, grid_y));
                }
            }

            // Создаём спрайт земли на уровне Z_MAP и запоминаем сущность по клетке
            let entity = crate::ecs::factory::create_sprite(
                &mut ecs.world, x, y, Z_MAP,
                tex_path, tex_pos, tex_count, 1.0, 1.0,
            );
            ecs.map_entities.insert((grid_x, grid_y), entity);
        }
        ecs.map_grid.push(grid_row);
    }
}

/// Загружает проходимые клетки из map.txt (для NPC pathfinding)
///
/// Возвращает множество клеток, по которым могут ходить покупатели.
/// Магазинные полы ("0"), трава и некоторые стены (двери) считаются проходимыми.
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

/// Находит точку спавна покупателей — самую нижнюю клетку `0` (пол магазина),
/// ближайшую к центру по X.
pub fn shopper_spawn_point() -> Node {
    let src = include_str!("../../map.txt");
    // Перебираем все клетки пола и ищем самую нижнюю (минимальный Y),
    // при равенстве Y — ближайшую к нулю по X
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

/// Сопоставляет токен карты с текстурой земли и кадром атласа.
/// Возвращает (путь к текстуре, позиция кадра в атласе, число кадров).
pub fn token_to_texture(token: &str) -> (&str, [i32; 2], [i32; 2]) {
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
        "q" => ("tex/map/wall.png", [2, 3], [5, 5]),
        "p" => ("tex/map/wall.png", [3, 3], [5, 5]),
        "i" => ("tex/map/wall.png", [4, 3], [5, 5]),
        //default
        _    => ("tex/map/floor.png", [0, 0], [2, 2]),
    }
}
