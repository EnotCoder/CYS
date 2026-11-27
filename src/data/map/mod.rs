// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

pub mod image_map;
pub mod pathfinding;

// ========================================================================
//  Загрузка карты из готовой картинки в ECS мир
// ========================================================================
//  Основная карта — один PNG 16 px на клетку (assets/map/*.png). Он рисуется
//  одним спрайтом, а image_map.rs по совпадению тайлов с атласами восстанавливает
//  игровые категории клеток. Подвал по-прежнему хранится в basement.txt и
//  читается построчно старым путём.

use crate::core::constants::{
    DOOR_TEX_CLOSED, DOOR_TEX_OPEN, DOOR_TEX_OPEN_1, DOOR_TEX_OPEN_2, DOOR_TEX_OPEN_3,
    DOOR_TILE_X_SCALE, DOOR_TILE_Y_OFFSET, DOOR_TILE_Y_SCALE, DOOR_TOKEN, DOOR_X_MAX, DOOR_X_MIN,
    DOOR_Y_MAX, DOOR_Y_MIN, SHOPPER_SPAWN_X,
    SIDEWALK_Y, WORLD_OFFSET_X, WORLD_OFFSET_Y, Z_MAP,
};
use crate::data::map::pathfinding::Node;
use crate::ecs::components::Season;
use crate::ecs::EcsAdapter;
use specs::WorldExt;
use std::collections::HashSet;
use std::io::BufRead;

/// Загружает карту-картинку: один фон + отдельные спрайты только для двери.
pub fn load_map_to_ecs(ecs: &mut EcsAdapter) {
    let map = image_map::cached_map();
    let season = *ecs.world.read_resource::<Season>();
    let sized = format!("{}@{}x{}", season.map_texture(), map.width, map.height);
    let cx = WORLD_OFFSET_X + (map.width as f32 - 1.0) * 0.5;
    let cy = WORLD_OFFSET_Y - (map.height as f32 - 1.0) * 0.5;
    let background = crate::ecs::factory::create_sprite(
        &mut ecs.world,
        cx,
        cy,
        Z_MAP,
        &sized,
        [0, 0],
        [1, 1],
        1.0,
        1.0,
    );
    ecs.map_background = Some(background);

    for (j, row) in map.cells.iter().enumerate() {
        let mut grid_row = Vec::with_capacity(row.len());
        for (i, cell) in row.iter().enumerate() {
            let token = cell.token;
            grid_row.push(token.to_string());
            register_cell(ecs, token, i, j);
            if is_door_token(token) {
                let x = i as f32 + WORLD_OFFSET_X;
                let y = -(j as f32) + WORLD_OFFSET_Y;
                let grid_x = (x + 0.5).floor() as i32;
                let grid_y = (y + 0.5).floor() as i32;
                spawn_map_tile(ecs, token, x, y, grid_x, grid_y);
            }
        }
        ecs.map_grid.push(grid_row);
    }
}

/// Загружает карту подвала из отдельного файла
pub fn load_basement_to_ecs(ecs: &mut EcsAdapter) {
    let bytes = crate::core::asset::load_bytes(crate::core::constants::BASEMENT_FILE)
        .expect("basement.txt not found!");
    load_map_from_reader(ecs, &bytes[..]);
}

/// Регистрирует категории клетки для размещения объектов и pathfinding.
fn register_cell(ecs: &mut EcsAdapter, token: &str, i: usize, j: usize) {
    let x = i as f32 + WORLD_OFFSET_X;
    let y = -(j as f32) + WORLD_OFFSET_Y;
    let grid_x = (x + 0.5).floor() as i32;
    let grid_y = (y + 0.5).floor() as i32;
    ecs.original_tokens.insert((grid_x, grid_y), token.to_string());
    if is_wall_tile(token) {
        ecs.wall_positions.insert((grid_x, grid_y));
    } else if is_floor_tile(token) {
        ecs.floor_positions.insert((grid_x, grid_y));
    }
    if crate::data::is_grass_token(token) {
        ecs.outdoor_positions.insert((grid_x, grid_y));
        ecs.flower_positions.insert((grid_x, grid_y));
    }
}

/// Меняет картинку фона при смене сезона.
pub fn set_map_season(ecs: &mut EcsAdapter, season: Season) {
    let Some(background) = ecs.map_background else {
        return;
    };
    let map = image_map::cached_map();
    let sized = format!("{}@{}x{}", season.map_texture(), map.width, map.height);
    ecs.update_sprite_texture(background, &sized);
}

/// Создаёт спрайт одного тайла карты и запоминает сущность в map_entities.
///
/// Единственная точка создания тайлов карты: её используют и обычная
/// загрузка (load_map_from_reader), и восстановление уровня из кэша
/// (GameScene::load_level). Если токен — клетка двери, дополнительно
/// выставляются её кадр атласа 2x2 и слой Z_DOOR (поверх покупателей).
/// Держать это в одном месте важно: когда логика двери расходилась по
/// двум вызовам, после похода в подвал дверь возвращалась на Z_MAP.
pub fn spawn_map_tile(
    ecs: &mut EcsAdapter,
    token: &str,
    world_x: f32,
    world_y: f32,
    grid_x: i32,
    grid_y: i32,
) -> specs::Entity {
    let season = *ecs.world.read_resource::<crate::ecs::components::Season>();
    let (tex_path, tex_pos, tex_count) = token_to_texture(token, season);
    let is_door = is_door_token(token);
    let sized_path;
    let path = if is_door {
        sized_path = format!("{}@{}x{}", tex_path, DOOR_TILE_X_SCALE, DOOR_TILE_Y_SCALE);
        sized_path.as_str()
    } else {
        tex_path
    };
    let tile_y = if is_door { world_y + DOOR_TILE_Y_OFFSET } else { world_y };
    let entity = crate::ecs::factory::create_sprite(
        &mut ecs.world,
        world_x,
        tile_y,
        Z_MAP,
        path,
        tex_pos,
        tex_count,
        1.0,
        1.0,
    );
    if is_door {
        ecs.apply_door_cell(entity, grid_x, grid_y);
    }
    ecs.map_entities.insert((grid_x, grid_y), entity);
    entity
}

/// Читает текстовую карту подвала построчно и превращает каждый токен в спрайт
fn load_map_from_reader(ecs: &mut EcsAdapter, reader: impl std::io::Read) {
    let reader = std::io::BufReader::new(reader);

    for (j, line) in reader.lines().flatten().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let mut grid_row: Vec<String> = Vec::new();
        for (i, token) in parts.iter().enumerate() {
            grid_row.push(token.to_string());
            register_cell(ecs, token, i, j);
            let x = i as f32 + WORLD_OFFSET_X;
            let y = -(j as f32) + WORLD_OFFSET_Y;
            let grid_x = (x + 0.5).floor() as i32;
            let grid_y = (y + 0.5).floor() as i32;
            spawn_map_tile(ecs, token, x, y, grid_x, grid_y);
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

/// Кадр атласа двери для клетки. Текстура двери 32x32 = 2x2 тайла, и каждая
/// из четырёх клеток карты обязана брать свой кадр, иначе проём соберётся
/// из четырёх одинаковых квадратов. token_to_texture позицию клетки не
/// видит, поэтому кадр проставляется здесь — и при загрузке карты, и при
/// смене состояния (см. update_door_textures).
///
/// По вертикали кадр берётся от нижнего края проёма: текстура сдвинута
/// на клетку вверх, чтобы перемычка двери встала вровень с горизонтальным
/// швом фасада. Поэтому Y = DOOR_Y_MIN (верх на экране) получает НИЖНЮЮ
/// строку атласа, а Y = DOOR_Y_MAX — верхнюю. По горизонтали порядок
/// обычный, слева направо.
pub fn door_frame_for_cell(x: i32, y: i32) -> [i32; 2] {
    [(x - DOOR_X_MIN) as i32, (DOOR_Y_MAX - y) as i32]
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

/// Загружает проходимые клетки из картинки карты (для NPC pathfinding)
///
/// Возвращает множество клеток, по которым могут ходить покупатели.
/// Проходимы: трава и тротуар ("."), пол магазина ("0") и дверной проём ("E").
pub fn load_walkable_cells() -> HashSet<Node> {
    let map = image_map::cached_map();
    let mut cells = HashSet::new();
    for (j, row) in map.cells.iter().enumerate() {
        for (i, cell) in row.iter().enumerate() {
            let token = cell.token;
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
    let map = image_map::cached_map();
    let walkable = load_walkable_cells();
    if walkable.contains(&preferred) {
        return preferred;
    }
    let mut best: Option<Node> = None;
    for (j, row) in map.cells.iter().enumerate() {
        let wy = -(j as f32) + WORLD_OFFSET_Y;
        if wy.round() as i32 != SIDEWALK_Y {
            continue;
        }
        for (i, _) in row.iter().enumerate() {
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
pub fn token_to_texture(
    token: &str,
    season: crate::ecs::components::Season,
) -> (&str, [i32; 2], [i32; 2]) {
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
        "E" => (
            "assets/tex/decor/regular/shop_door/shop_door.png",
            [0, 0],
            [2, 2],
        ),
        "q" => ("assets/tex/map/wall.png", [2, 3], [5, 5]),
        "p" => ("assets/tex/map/wall.png", [3, 3], [5, 5]),
        "i" => ("assets/tex/map/wall.png", [4, 3], [5, 5]),

        //default
        _ => ("assets/tex/map/floor.png", [0, 0], [2, 2]),
    }
}
