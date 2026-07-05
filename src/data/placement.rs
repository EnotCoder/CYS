use specs::WorldExt;
use crate::EcsAdapter;
use super::Slot;

pub fn is_carpet_name(name: &str) -> bool {
    crate::constants::CARPET_NAMES.contains(&name)
}

pub fn is_wall_decor_name(name: &str) -> bool {
    crate::constants::INV_WALLDECOR.contains(&name)
}

pub fn is_outdoor_name(name: &str) -> bool {
    crate::constants::OUTDOOR_NAMES.contains(&name)
}

pub fn is_flower_name(name: &str) -> bool {
    crate::constants::FLOWER_NAMES.contains(&name)
}

fn is_grass_token(token: &str) -> bool {
    matches!(token, "." | "@" | "*" | "m" | "f" | "~" | "l" | "1" | "2" | "3" | "4" | "5" | "6")
}

fn update_walls_around(ecs: &mut EcsAdapter, gx: i32, gy: i32) {
    let dirs: [(i32, i32, &str); 8] = [
        (0, -1, "&"), (0, 1, "&"), (-1, 0, "/"), (1, 0, "|"),
        (-1, -1, "p"), (1, -1, "i"), (-1, 1, "/"), (1, 1, "|"),
    ];
    for &(dx, dy, wall_tok) in &dirs {
        let nx = gx + dx;
        let ny = gy + dy;
        let file_col = nx + 21;
        let file_row = 14 - ny;
        if file_row < 0 || file_row >= ecs.map_grid.len() as i32 { continue; }
        if file_col < 0 || file_col >= ecs.map_grid[file_row as usize].len() as i32 { continue; }
        let token = ecs.map_grid[file_row as usize][file_col as usize].clone();
        if !is_grass_token(&token) { continue; }
        ecs.map_grid[file_row as usize][file_col as usize] = wall_tok.to_string();
        ecs.flower_positions.remove(&(nx, ny));
        if let Some(&map_entity) = ecs.map_entities.get(&(nx, ny)) {
            ecs.update_sprite_texture(map_entity, "tex/map/wall.png");
            let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
            if let Some(sprite) = sprites.get_mut(map_entity) {
                let (tf, tc) = wall_frame_count(wall_tok);
                sprite.texture_frame = tf;
                sprite.texture_count = tc;
            }
        }
    }
}

fn refresh_walls_around(ecs: &mut EcsAdapter, gx: i32, gy: i32) {
    let dirs: [(i32, i32); 8] = [
        (0, -1), (0, 1), (-1, 0), (1, 0),
        (-1, -1), (1, -1), (-1, 1), (1, 1),
    ];
    for &(dx, dy) in &dirs {
        let nx = gx + dx;
        let ny = gy + dy;
        let file_col = nx + 21;
        let file_row = 14 - ny;
        if file_row < 0 || file_row >= ecs.map_grid.len() as i32 { continue; }
        if file_col < 0 || file_col >= ecs.map_grid[file_row as usize].len() as i32 { continue; }
        let token = ecs.map_grid[file_row as usize][file_col as usize].clone();
        if token == "0" || is_grass_token(&token) { continue; }
        if let Some(new_token) = recompute_wall_token(ecs, nx, ny) {
            if new_token != token {
                ecs.map_grid[file_row as usize][file_col as usize] = new_token.clone();
                if let Some(&map_entity) = ecs.map_entities.get(&(nx, ny)) {
                    ecs.update_sprite_texture(map_entity, "tex/map/wall.png");
                    let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
                    if let Some(sprite) = sprites.get_mut(map_entity) {
                        let (tf, tc) = wall_frame_count(&new_token);
                        sprite.texture_frame = tf;
                        sprite.texture_count = tc;
                    }
                }
            }
        } else {
            revert_to_grass(ecs, nx, ny, file_row, file_col);
        }
    }
}

fn revert_to_grass(ecs: &mut EcsAdapter, nx: i32, ny: i32, file_row: i32, file_col: i32) {
    ecs.floor_positions.remove(&(nx, ny));
    let original = ecs.original_tokens.get(&(nx, ny)).cloned().unwrap_or_else(|| ".".to_string());
    ecs.map_grid[file_row as usize][file_col as usize] = original.clone();
    if is_grass_token(&original) {
        ecs.outdoor_positions.insert((nx, ny));
        ecs.flower_positions.insert((nx, ny));
    }
    let (tex, frame, count) = crate::map::token_to_texture(&original);
    if let Some(&map_entity) = ecs.map_entities.get(&(nx, ny)) {
        ecs.update_sprite_texture(map_entity, tex);
        let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
        if let Some(sprite) = sprites.get_mut(map_entity) {
            sprite.texture_frame = frame;
            sprite.texture_count = count;
        }
    }
}

fn recompute_wall_token(ecs: &EcsAdapter, wx: i32, wy: i32) -> Option<String> {
    let f = |x: i32, y: i32| ecs.floor_positions.contains(&(x, y));
    let s = f(wx, wy+1); let n = f(wx, wy-1);
    let e = f(wx+1, wy); let w = f(wx-1, wy);
    let se = f(wx+1, wy+1); let sw = f(wx-1, wy+1);
    let ne = f(wx+1, wy-1); let nw = f(wx-1, wy-1);
    let count = [s, n, e, w, se, sw, ne, nw].iter().filter(|&&x| x).count();
    if count == 0 { return None; }
    if count == 1 {
        if s { return Some("&".to_string()); } if n { return Some("&".to_string()); }
        if e { return Some("/".to_string()); } if w { return Some("|".to_string()); }
        if se { return Some("p".to_string()); } if sw { return Some("i".to_string()); }
        if ne { return Some("/".to_string()); } if nw { return Some("|".to_string()); }
    }
    if s || n { return Some("&".to_string()); }
    if e || ne { return Some("/".to_string()); }
    if w || nw { return Some("|".to_string()); }
    if se { return Some("p".to_string()); } if sw { return Some("i".to_string()); }
    None
}

fn wall_frame_count(token: &str) -> ([i32; 2], [i32; 2]) {
    match token {
        "=" => ([0, 0], [5, 5]), "-" => ([0, 1], [5, 5]),
        "^" => ([1, 0], [5, 5]), "&" => ([2, 2], [5, 5]),
        "/" => ([0, 2], [5, 5]), "|" => ([1, 2], [5, 5]),
        "(" => ([0, 3], [5, 5]), "{" => ([0, 4], [5, 5]),
        ")" => ([1, 3], [5, 5]), "}" => ([1, 4], [5, 5]),
        "[" => ([2, 0], [5, 5]), "]" => ([4, 0], [5, 5]),
        ":" => ([2, 1], [5, 5]), ";" => ([4, 1], [5, 5]),
        "o" => ([3, 0], [5, 5]), "%" => ([3, 1], [5, 5]),
        "p" => ([3, 3], [5, 5]), "i" => ([4, 3], [5, 5]),
        _ => ([0, 0], [5, 5]),
    }
}

pub fn add(ecs: &mut EcsAdapter, slots: &mut Vec<Slot>, act_slot: i32, gx: i32, gy: i32) {
    let active_slot = &slots[act_slot as usize].obj;
    let is_carpet = is_carpet_name(active_slot.name);
    let is_wall_decor = is_wall_decor_name(active_slot.name);
    let is_outdoor = is_outdoor_name(active_slot.name);
    let is_flower = is_flower_name(active_slot.name);
    let is_floor = active_slot.name == "floor";

    if !ecs.can_place_at(
        gx, gy,
        active_slot.width, active_slot.height,
        is_carpet, is_wall_decor, is_outdoor, is_flower, is_floor,
    ) {
        return;
    }

    ecs.clear_cursor_preview();

    if is_floor && !ecs.map_grid.is_empty() {
        for i in 0..active_slot.width {
            for j in 0..active_slot.height {
                let cx = gx + i;
                let cy = gy + j;
                let file_col = cx + 21;
                let file_row = 14 - cy;
                if file_row >= 0 && file_row < ecs.map_grid.len() as i32 &&
                   file_col >= 0 && file_col < ecs.map_grid[file_row as usize].len() as i32
                {
                    let token = ecs.map_grid[file_row as usize][file_col as usize].clone();
                    if token != "0" {
                        ecs.original_tokens.entry((cx, cy)).or_insert_with(|| token.clone());
                        ecs.map_grid[file_row as usize][file_col as usize] = "0".to_string();
                        ecs.floor_positions.insert((cx, cy));
                        ecs.outdoor_positions.remove(&(cx, cy));
                        ecs.flower_positions.remove(&(cx, cy));
                        if let Some(&map_entity) = ecs.map_entities.get(&(cx, cy)) {
                            ecs.update_sprite_texture(map_entity, "tex/map/floor.png");
                            let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
                            if let Some(sprite) = sprites.get_mut(map_entity) {
                                sprite.texture_frame = [0, 0];
                                sprite.texture_count = [2, 2];
                            }
                        }
                        update_walls_around(ecs, cx, cy);
                        refresh_walls_around(ecs, cx, cy);
                    }
                }
            }
        }
        // map.txt не сохраняем — пол и стены только в памяти
        return;
    }

    let group_id = ecs.add_group_object(
        gx, gy,
        active_slot.width, active_slot.height,
        active_slot.path,
        active_slot.texture_frame,
        active_slot.texture_count,
        is_carpet,
        active_slot.animated,
        active_slot.frame_paths,
        false, // is_floor is always false here
    );

    let first = {
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
        groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
    };
    if let Some(entity) = first {
        use specs::WorldExt;
        ecs.world.write_storage::<crate::ObjectTag>().insert(entity, crate::ObjectTag {
            name: active_slot.name.to_string(),
        }).ok();
        if active_slot.name == "box" {
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food: 20,
            }).ok();
        } else if active_slot.name == "rack" {
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food: 15,
            }).ok();
        } else if active_slot.name == "fence" || active_slot.name == "street_fence" {
            ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent { name: active_slot.name.to_string() }).ok();
        }
    }
}

pub fn remove(ecs: &mut EcsAdapter, gx: i32, gy: i32) -> bool {
    if let Some(group_id) = ecs.find_group_at_position(gx, gy) {
        ecs.delete_group(group_id);
        return true;
    }
    let file_col = gx + 21;
    let file_row = 14 - gy;
    if file_row >= 0 && file_row < ecs.map_grid.len() as i32 &&
       file_col >= 0 && file_col < ecs.map_grid[file_row as usize].len() as i32
    {
        let token = &ecs.map_grid[file_row as usize][file_col as usize];
        if token == "0" {
            revert_to_grass(ecs, gx, gy, file_row, file_col);
            refresh_walls_around(ecs, gx, gy);
            return true;
        }
        if !is_grass_token(token) {
            let original = ecs.original_tokens.get(&(gx, gy));
            if original.map_or(false, |o| is_grass_token(o)) {
                revert_to_grass(ecs, gx, gy, file_row, file_col);
                refresh_walls_around(ecs, gx, gy);
                return true;
            }
        }
    }
    false
}
