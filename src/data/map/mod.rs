// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

pub mod pathfinding;

// ========================================================================
//  Загрузка карты из map.txt в ECS мир
// ========================================================================
//  map.txt описывает уровень строками токенов, разделённых пробелами.
//  Каждый токен кодирует клетку: "=" / "-" — стены, "0" и его паркетные
//  варианты ("P" "_" "Q" "A" "Z" "," "X" "D") — пол магазина,
//  "." и прочие — трава снаружи, "/" / "|" / "&" — стены, на которые можно
//  ставить предметы, "^" / "[" / "]" и др. — декоративные стены и окна.
//  Здесь же происходит разбор файла и создание ECS-сущностей земли.

use std::collections::HashSet;
use std::io::BufRead;
use specs::WorldExt;
use crate::ecs::EcsAdapter;
use crate::core::constants::{
    WORLD_OFFSET_X, WORLD_OFFSET_Y, Z_MAP,
    DOOR_TOKEN, DOOR_X_MIN, DOOR_X_MAX, DOOR_Y_MIN, DOOR_Y_MAX,
    DOOR_TEX_CLOSED, DOOR_TEX_OPEN, DOOR_TEX_OPEN_1, DOOR_TEX_OPEN_2, DOOR_TEX_OPEN_3,
    SIDEWALK_Y, SHOPPER_SPAWN_X,
};
use crate::data::map::pathfinding::Node;

/// Загружает карту из файла и создаёт для каждой клетки ECS-сущность
pub fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let bytes = crate::core::asset::load_bytes(crate::core::constants::MAP_FILE).expect("map.txt not found!");
    load_map_from_reader(ecs, &bytes[..], false);
}

/// Загружает карту подвала из отдельного файла
pub fn load_basement_to_ecs(ecs: &mut EcsAdapter) {
    let bytes = crate::core::asset::load_bytes(crate::core::constants::BASEMENT_FILE).expect("basement.txt not found!");
    load_map_from_reader(ecs, &bytes[..], true);
}

/// Читает текстовую карту построчно и превращает каждый токен в спрайт-сущность
fn load_map_from_reader(ecs: &mut EcsAdapter, reader: impl std::io::Read, _is_basement: bool) {
    let reader = std::io::BufReader::new(reader);
    let season = *ecs.world.read_resource::<crate::ecs::components::Season>();

    // j — номер строки (ось Y), i — позиция в строке (ось X)
    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let mut grid_row: Vec<String> = Vec::new();
        for (i, token) in parts.iter().enumerate() {
            grid_row.push(token.to_string());
            let (tex_path, tex_pos, tex_count) = token_to_texture(token, season);

            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;
            let grid_x = (x + 0.5).floor() as i32;
            let grid_y = (y + 0.5).floor() as i32;

            // Сохраняем исходный токен клетки, чтобы уметь восстанавливать уровень
            ecs.original_tokens.insert((grid_x, grid_y), token.to_string());

            // Токены травы/улицы — помечаем клетки как outdoor и пригодные к посадке цветов
            let is_grass = crate::data::is_grass_token(token);
            if is_wall_tile(token) {
                ecs.wall_positions.insert((grid_x, grid_y));
            } else if is_floor_tile(token) {
                ecs.floor_positions.insert((grid_x, grid_y));
            }
            if is_grass {
                ecs.outdoor_positions.insert((grid_x, grid_y));
                ecs.flower_positions.insert((grid_x, grid_y));
            }

            // Создаём спрайт земли на уровне Z_MAP и запоминаем сущность по клетке
            let entity = crate::ecs::factory::create_sprite(
                &mut ecs.world, x, y, Z_MAP,
                tex_path, tex_pos, tex_count, 1.0, 1.0,
            );
            // Дверь — 2x2 атлас, и каждой её клетке нужен свой кадр,
            // иначе проём соберётся из четырёх одинаковых квадратов.
            if is_door_token(token) {
                ecs.set_door_cell_frame(entity, door_frame_for_cell(grid_x, grid_y));
            }
            ecs.map_entities.insert((grid_x, grid_y), entity);
        }
        ecs.map_grid.push(grid_row);
    }
}

/// Токены пола магазина: "0" (центр) и варианты краёв/углов паркета
/// ("P" "_" "Q" "A" "Z" "," "X" "D"). На них можно ставить предметы.
pub fn is_floor_tile(token: &str) -> bool {
    matches!(token, "0" | "P" | "_" | "Q" | "A" | "Z" | "," | "X" | "D")
}

/// Токены стен магазина: "=", "-", "W", "S" и др. На них можно вешать настенный декор.
pub fn is_wall_tile(token: &str) -> bool {
    matches!(token, "W" | "S")
}

/// Токены дверного проёма магазина ("E"). Это не пол и не стена: на дверь
/// ничего нельзя поставить, но сквозь неё проходят покупатели, и её
/// нельзя перезаписывать пересчётом стен (см. refresh_walls_around).
pub fn is_door_token(token: &str) -> bool {
    token == DOOR_TOKEN
}

/// Клетки дверного проёма (2x2) в мировых координатах.
pub fn door_cells() -> Vec<(i32, i32)> {
    let mut cells = Vec::with_capacity(4);
    for y in DOOR_Y_MIN..=DOOR_Y_MAX {
        for x in DOOR_X_MIN..=DOOR_X_MAX {
            cells.push((x, y));
        }
    }
    cells
}

/// Кадр атласа двери для клетки — её смещение относительно левого верхнего
/// угла проёма. Текстура двери 32x32 = 2x2 тайла, и каждая из четырёх
/// клеток карты обязана брать свой кадр, иначе проём соберётся из
/// четырёх одинаковых квадратов. token_to_texture позицию клетки не видит,
/// поэтому кадр проставляется здесь — и при загрузке карты, и при смене
/// состояния (см. update_door_textures).
pub fn door_frame_for_cell(x: i32, y: i32) -> [i32; 2] {
    [(x - DOOR_X_MIN) as i32, (y - DOOR_Y_MIN) as i32]
}

/// Текстура двери для шага анимации: 0 = закрыта, 3 = полностью открыта.
/// Шаги 1..3 — кадры распахивания, они же используются в обратном порядке
/// при закрывании.
pub fn door_texture_for_step(step: i32) -> &'static str {
    match step {
        i32::MIN..=0 => DOOR_TEX_CLOSED,
        1 => DOOR_TEX_OPEN_1,
        2 => DOOR_TEX_OPEN_2,
        3 => DOOR_TEX_OPEN_3,
        _ => DOOR_TEX_OPEN,
    }
}

/// Загружает проходимые клетки из map.txt (для NPC pathfinding)
///
/// Возвращает множество клеток, по которым могут ходить покупатели.
/// Проходимы: трава (включая декоративную "H"/"K"), тротуарные тени "1".."6",
/// пол магазина "0", паркетная рамка пола (P _ Q A D Z , X — по ней покупатель
/// заходит в магазин) и дверной проём "E".
pub fn load_walkable_cells() -> HashSet<Node> {
    let src = include_str!("../../../assets/map.txt");
    let mut cells = HashSet::new();
    for (j, line) in src.lines().enumerate() {
        for (i, token) in line.split_whitespace().enumerate() {
            // Проходимо: трава (в т.ч. декоративные "H"/"K"), тротуарные тени
            // "1".."6", пол "0", паркетная рамка пола и дверной проём "E".
            // "!"/"~" оставлены на случай правок карты.
            let passable = is_floor_tile(token)
                || is_door_token(token)
                || matches!(token, "!" | "~")
                || crate::data::is_grass_token(token);
            if passable {
                let wx = i as f32 + WORLD_OFFSET_X;
                let wy = -(j as f32) + WORLD_OFFSET_Y;
                cells.insert(Node::from_world(wx, wy));
            }
        }
    }
    cells
}

/// Точка входа покупателей с улицы — клетка тротуара напротив магазина,
/// восточнее двери. Оттуда покупатель идёт вдоль фасада и заворачивает в
/// дверь. Если клетка из константы почему-то не проходима (правка карты),
/// берётся ближайшая проходимая клетка тротуара — падать нельзя, иначе
/// не будет ни одного покупателя.
pub fn shopper_spawn_point() -> Node {
    let preferred = Node::new(SHOPPER_SPAWN_X, SIDEWALK_Y);
    let walkable = load_walkable_cells();
    if walkable.contains(&preferred) {
        return preferred;
    }
    let mut best: Option<Node> = None;
    for j in 0..100 {
        let wy = -(j as f32) + WORLD_OFFSET_Y;
        if wy.round() as i32 != SIDEWALK_Y {
            continue;
        }
        let line = include_str!("../../../assets/map.txt").lines().nth(j).unwrap_or("");
        for (i, _) in line.split_whitespace().enumerate() {
            let node = Node::new(i as i32 + WORLD_OFFSET_X as i32, SIDEWALK_Y);
            if !walkable.contains(&node) {
                continue;
            }
            // Ближайшая к магазину (меньше |X|), чтобы не уходить на край карты.
            let better = best.map_or(true, |cur: Node| {
                node.x.abs() < cur.x.abs() || (node.x.abs() == cur.x.abs() && node.x > cur.x)
            });
            if better {
                best = Some(node);
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.unwrap_or(preferred)
}

/// Сопоставляет токен карты с текстурой земли и кадром атласа.
/// Возвращает (путь к текстуре, позиция кадра в атласе, число кадров).
pub fn token_to_texture(token: &str, season: crate::ecs::components::Season) -> (&str, [i32; 2], [i32; 2]) {
    let grass = season.grass_texture();
    match token {
        //grass
        "." => (grass, [0, 0], [4, 6]),
        "@" => (grass, [0, 2], [4, 6]),
        "*" => (grass, [2, 2], [4, 6]),
        "m" => (grass, [3, 2], [4, 6]),
        "f" => (grass, [2, 3], [4, 6]),
        "~" => (grass, [1, 2], [4, 6]),
        "l" => (grass, [0, 3], [4, 6]),
        "H" => (grass, [1, 3], [4, 6]),
        "K" => (grass, [3, 1], [4, 6]),

        //shadow
        "1" => (grass, [0, 1], [4, 6]),
        "2" => (grass, [1, 1], [4, 6]),
        "3" => (grass, [2, 1], [4, 6]),
        "4" => (grass, [1, 0], [4, 6]),
        "5" => (grass, [2, 0], [4, 6]),
        "6" => (grass, [3, 0], [4, 6]),
        
        //floor
        "P" => ("assets/tex/map/floor.png", [0, 0], [3, 3]),
        "Q" => ("assets/tex/map/floor.png", [2, 0], [3, 3]),
        "Z" => ("assets/tex/map/floor.png", [0, 2], [3, 3]),
        "X" => ("assets/tex/map/floor.png", [2, 2], [3, 3]),
        "," => ("assets/tex/map/floor.png", [1, 2], [3, 3]),
        "_" => ("assets/tex/map/floor.png", [1, 0], [3, 3]),
        "A" => ("assets/tex/map/floor.png", [0, 1], [3, 3]),
        "D" => ("assets/tex/map/floor.png", [2, 1], [3, 3]),
        "0" => ("assets/tex/map/floor.png", [1, 1], [3, 3]),

        //wall
        "=" => ("assets/tex/map/wall.png", [0, 0], [5, 5]),
        "-" => ("assets/tex/map/wall.png", [0, 1], [5, 5]),
        "h" => ("assets/tex/map/wall.png", [0, 0], [5, 5]),
        "d" => ("assets/tex/map/wall.png", [0, 1], [5, 5]),
        "^" => ("assets/tex/map/wall.png", [1, 0], [5, 5]),
        "&" => ("assets/tex/map/wall.png", [1, 1], [5, 5]),
        "W" => ("assets/tex/map/wall.png", [0, 2], [5, 5]),
        "S" => ("assets/tex/map/wall.png", [0, 3], [5, 5]),

        "/" => (grass, [0, 4], [4, 6]),
        "|" => (grass, [1, 4], [4, 6]),
        "(" => (grass, [2, 4], [4, 6]),
        "{" => (grass, [2, 5], [4, 6]),
        ")" => (grass, [3, 4], [4, 6]),
        "}" => (grass, [3, 5], [4, 6]),
        
        //window
        "[" => ("assets/tex/map/wall.png", [2, 0], [5, 5]),
        "]" => ("assets/tex/map/wall.png", [4, 0], [5, 5]),
        ":" => ("assets/tex/map/wall.png", [2, 1], [5, 5]),
        ";" => ("assets/tex/map/wall.png", [4, 1], [5, 5]),
        "o" => ("assets/tex/map/wall.png", [3, 0], [5, 5]),
        "%" => ("assets/tex/map/wall.png", [3, 1], [5, 5]),

        //door — дверной проём магазина. Текстура 32x32 = 2x2 тайла, поэтому
        //каждая из четырёх клеток проёма берёт свой кадр атласа и вместе
        //собирает цельный проём. Состояние (открыта/закрыта) переключает
        //update_door_textures, подменяя путь к текстуре.
        "E" => ("assets/tex/decor/regular/shop_door/shop_door.png", [0, 0], [2, 2]),
        "q" => ("assets/tex/map/wall.png", [2, 3], [5, 5]),
        "p" => ("assets/tex/map/wall.png", [3, 3], [5, 5]),
        "i" => ("assets/tex/map/wall.png", [4, 3], [5, 5]),

        //default
        _    => ("assets/tex/map/floor.png", [0, 0], [2, 2]),
    }
}
