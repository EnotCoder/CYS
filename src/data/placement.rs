use specs::WorldExt;
use crate::EcsAdapter;
use super::Slot;
use crate::ecs::components::BasementPlaced;

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
                ecs.map_grid[file_row as usize][file_col as usize] = new_token.to_string();
                if new_token == "&" {
                    let is_bottom = file_row > 0
                        && ecs.map_grid.get(file_row as usize - 1)
                            .and_then(|row| row.get(file_col as usize))
                            .map_or(false, |t| t == "0");
                    if is_bottom {
                        ecs.floor_placeable_positions.remove(&(nx, ny));
                    } else {
                        ecs.floor_placeable_positions.insert((nx, ny));
                    }
                } else if matches!(new_token, "/" | "|") {
                    ecs.floor_placeable_positions.insert((nx, ny));
                } else {
                    ecs.floor_placeable_positions.remove(&(nx, ny));
                }
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
    ecs.floor_placed_positions.remove(&(nx, ny));
    let original = ecs.original_tokens.get(&(nx, ny)).cloned().unwrap_or_else(|| ".".to_string());
    ecs.map_grid[file_row as usize][file_col as usize] = original.clone();
    if original == "&" {
        let is_bottom = file_row > 0
            && ecs.map_grid.get(file_row as usize - 1)
                .and_then(|row| row.get(file_col as usize))
                .map_or(false, |t| t == "0");
        if is_bottom {
            ecs.floor_placeable_positions.remove(&(nx, ny));
        } else {
            ecs.floor_placeable_positions.insert((nx, ny));
        }
    } else if !matches!(original.as_str(), "/" | "|" | ".") {
        ecs.floor_placeable_positions.remove(&(nx, ny));
    }
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

fn recompute_wall_token(ecs: &EcsAdapter, wx: i32, wy: i32) -> Option<&'static str> {
    let f = |x: i32, y: i32| ecs.floor_positions.contains(&(x, y));
    let s = f(wx, wy+1); let n = f(wx, wy-1);
    let e = f(wx+1, wy); let w = f(wx-1, wy);
    let se = f(wx+1, wy+1); let sw = f(wx-1, wy+1);
    let ne = f(wx+1, wy-1); let nw = f(wx-1, wy-1);
    let count = [s, n, e, w, se, sw, ne, nw].iter().filter(|&&x| x).count();
    if count == 0 { return None; }
    if count == 1 {
        if s { return Some("&"); } if n { return Some("&"); }
        if e { return Some("/"); } if w { return Some("|"); }
        if se { return Some("p"); } if sw { return Some("i"); }
        if ne { return Some("/"); } if nw { return Some("|"); }
    }
    if s || n { return Some("&"); }
    if e || ne { return Some("/"); }
    if w || nw { return Some("|"); }
    if se { return Some("p"); } if sw { return Some("i"); }
    None
}

fn wall_frame_count(token: &str) -> ([i32; 2], [i32; 2]) {
    match token {
        "=" => ([0, 0], [5, 5]), "-" => ([0, 1], [5, 5]),
        "^" => ([1, 0], [5, 5]), "&" => ([1, 1], [5, 5]),
        "/" => ([0, 2], [5, 5]), "|" => ([1, 2], [5, 5]),
        "(" => ([0, 3], [5, 5]), "{" => ([0, 4], [5, 5]),
        ")" => ([1, 3], [5, 5]), "}" => ([1, 4], [5, 5]),
        "[" => ([2, 0], [5, 5]), "]" => ([4, 0], [5, 5]),
        ":" => ([2, 1], [5, 5]), ";" => ([4, 1], [5, 5]),
        "o" => ([3, 0], [5, 5]), "%" => ([3, 1], [5, 5]),
        "q" => ([2, 3], [5, 5]),
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

    // В подвале нельзя ставить кассу, стеллаж и лестницу
    if ecs.current_level == -1 {
        if active_slot.name == "cassa" || active_slot.name == "rack" || active_slot.name == "basement" {
            return;
        }
    }

    // Только один подвал на магазин
    if active_slot.name == "basement" && ecs.world.read_resource::<BasementPlaced>().0 {
        return;
    }

    if !ecs.can_place_at(
        gx, gy,
        active_slot.width, active_slot.height,
        is_carpet, is_wall_decor, is_outdoor, is_flower,
    ) {
        return;
    }

    ecs.clear_cursor_preview();

    let group_id = ecs.add_group_object(
        gx, gy,
        active_slot.width, active_slot.height,
        active_slot.path,
        active_slot.texture_frame,
        active_slot.texture_count,
        is_carpet,
        active_slot.animated,
        active_slot.frame_paths,
    );

    let first = {
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
        groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
    };
    if let Some(entity) = first {
        ecs.world.write_storage::<crate::ObjectTag>().insert(entity, crate::ObjectTag {
            name: active_slot.name.to_string(),
        }).ok();
        if active_slot.name == "basement" {
            ecs.world.write_resource::<BasementPlaced>().0 = true;
        } else if active_slot.name == "box" {
            let max_food = ecs.world.read_resource::<crate::script::config::BalanceConfig>().max_food_box;
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food,
            }).ok();
        } else if active_slot.name == "rack" {
            let max_food = ecs.world.read_resource::<crate::script::config::BalanceConfig>().max_food_rack;
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food,
            }).ok();
        } else if active_slot.name == "candies" {
            let cfg = ecs.world.read_resource::<crate::script::config::BalanceConfig>();
            let (max_food, start) = (cfg.max_food_candies, cfg.candies_start_food);
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: start,
                max_food,
            }).ok();
        } else if active_slot.name == "fence" || active_slot.name == "street_fence" {
            ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent { name: active_slot.name.to_string() }).ok();
        }
    }
}

pub fn remove(ecs: &mut EcsAdapter, gx: i32, gy: i32) -> bool {
    if let Some(group_id) = ecs.find_group_at_position(gx, gy) {
        let entity = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = entity {
            let is_basement = ecs.world.read_storage::<crate::ObjectTag>().get(entity).map(|t| t.name == "basement").unwrap_or(false);
            if is_basement {
                ecs.world.write_resource::<BasementPlaced>().0 = false;
            }
        }
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
