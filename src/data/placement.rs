// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::WorldExt;
use crate::EcsAdapter;
use super::Slot;
use crate::ecs::components::{BasementPlaced, Money};

// ========================================================================
//  Правила размещения объектов: категории, стены, трава
// ========================================================================

// Категории объектов определяют, куда их можно ставить
// (пол, стены, улица, клумбы) — проверки живут в EcsAdapter.

pub fn is_carpet_name(name: &str) -> bool {
    crate::core::constants::CARPET_NAMES.contains(&name)
}

pub fn is_wall_decor_name(name: &str) -> bool {
    crate::core::constants::INV_WALLDECOR.contains(&name)
}

pub fn is_outdoor_name(name: &str) -> bool {
    crate::core::constants::OUTDOOR_NAMES.contains(&name)
}

pub fn is_flower_name(name: &str) -> bool {
    crate::core::constants::FLOWER_NAMES.contains(&name)
}

pub fn is_light_name(name: &str) -> bool {
    crate::core::constants::INV_LIGHT.contains(&name)
}

// Привязывает PointLight к сущности по имени объекта.
// Используется при установке объекта и при восстановлении уровня из сохранения.
pub fn attach_point_light(ecs: &EcsAdapter, entity: specs::Entity, name: &str) {
    use crate::ecs::components::PointLight;
    let light = match name {
        "street_ice_cream" => PointLight { color: [1.0, 0.8, 0.6], intensity: 0.8, radius: 4.0 },
        "arcade_machine" => PointLight { color: [0.6, 0.8, 1.0], intensity: 0.8, radius: 3.0 },
        "candies" => PointLight { color: [1.0, 0.6, 1.0], intensity: 0.6, radius: 2.0 },
        "lamp" => PointLight { color: [1.0, 0.95, 0.8], intensity: 1.0, radius: 5.0 },
        "big_lamp" => PointLight { color: [1.0, 0.95, 0.8], intensity: 1.4, radius: 7.0 },
        _ => return,
    };
    ecs.world.write_storage::<PointLight>().insert(entity, light).ok();
}

// Токеры «травы» — свободный открытый грунт, на который можно сажать
// уличные объекты и цветы (например "." — трава, "f" — цветок и т.д.)
fn is_grass_token(token: &str) -> bool {
    matches!(token, "." | "@" | "*" | "m" | "f" | "~" | "l" | "1" | "2" | "3" | "4" | "5" | "6")
}

// Пересчитать вид стен вокруг клетки (gx, gy) после установки/удаления.
// Стены рисуются так, чтобы стыковаться с соседними стенами и полом —
// это зависит от того, какие соседние клетки являются полом (floor_positions).
fn refresh_walls_around(ecs: &mut EcsAdapter, gx: i32, gy: i32) {
    let dirs: [(i32, i32); 8] = [
        (0, -1), (0, 1), (-1, 0), (1, 0),
        (-1, -1), (1, -1), (-1, 1), (1, 1),
    ];
    for &(dx, dy) in &dirs {
        let nx = gx + dx;
        let ny = gy + dy;
        // Переводим игровые координаты в индексы сетки карты (карта читается из файла).
        let file_col = nx + 21;
        let file_row = 14 - ny;
        if file_row < 0 || file_row >= ecs.map_grid.len() as i32 { continue; }
        if file_col < 0 || file_col >= ecs.map_grid[file_row as usize].len() as i32 { continue; }
        let token = ecs.map_grid[file_row as usize][file_col as usize].clone();
        // Пустота и трава стены не образуют.
        if token == "0" || is_grass_token(&token) { continue; }
        // Пересчитываем вид стены; None — значит без пола рядом стена не нужна.
        if let Some(new_token) = recompute_wall_token(ecs, nx, ny) {
            if new_token != token {
                ecs.map_grid[file_row as usize][file_col as usize] = new_token.to_string();
                // На стену ("&") можно ставить напольные предметы, если она
                // стоит на клетке пола — отслеживаем это в списке позиций.
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
                // Обновляем спрайт стены на новый визуальный кадр.
                if let Some(&map_entity) = ecs.map_entities.get(&(nx, ny)) {
                    ecs.update_sprite_texture(map_entity, "assets/tex/map/wall.png");
                    let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
                    if let Some(sprite) = sprites.get_mut(map_entity) {
                        let (tf, tc) = wall_frame_count(&new_token);
                        sprite.texture_frame = tf;
                        sprite.texture_count = tc;
                    }
                }
            }
        } else {
            // Рядом нет пола — стену убираем, возвращаем исходную траву.
            revert_to_grass(ecs, nx, ny, file_row, file_col);
        }
    }
}

// Вернуть клетку к исходному состоянию (обычно трава), восстановив
// позиции для уличных объектов/цветов и спрайт из original_tokens.
fn revert_to_grass(ecs: &mut EcsAdapter, nx: i32, ny: i32, file_row: i32, file_col: i32) {
    ecs.floor_positions.remove(&(nx, ny));
    ecs.floor_placed_positions.remove(&(nx, ny));
    let original = ecs.original_tokens.get(&(nx, ny)).cloned().unwrap_or_else(|| ".".to_string());
    ecs.map_grid[file_row as usize][file_col as usize] = original.clone();
    // Восстанавливаем, можно ли на эту клетку ставить напольные предметы.
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
    // Исходная трава снова принимает уличные объекты и цветы.
    if is_grass_token(&original) {
        ecs.outdoor_positions.insert((nx, ny));
        ecs.flower_positions.insert((nx, ny));
    }
    let (tex, frame, count) = crate::data::map::token_to_texture(&original);
    if let Some(&map_entity) = ecs.map_entities.get(&(nx, ny)) {
        ecs.update_sprite_texture(map_entity, tex);
        let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
        if let Some(sprite) = sprites.get_mut(map_entity) {
            sprite.texture_frame = frame;
            sprite.texture_count = count;
        }
    }
}

// Определить токен стены по соседям: смотрим, где вокруг есть пол (f),
// и возвращаем код вида стены ("&" — сплошная, "/" и "|" — углы и т.д.)
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

// Кадр в атласе стены (5x5 кадров) для конкретного токена вида стены.
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

// Поставить активный объект из слота в клетку (gx, gy).
pub fn add(ecs: &mut EcsAdapter, slots: &mut Vec<Slot>, act_slot: i32, gx: i32, gy: i32) {
    let active_slot = &slots[act_slot as usize].obj;
    // Заранее определяем категорию объекта — от неё зависят правила размещения.
    let is_carpet = is_carpet_name(active_slot.name);
    let is_wall_decor = is_wall_decor_name(active_slot.name);
    let is_outdoor = is_outdoor_name(active_slot.name);
    let is_flower = is_flower_name(active_slot.name);
    let is_light = is_light_name(active_slot.name);

    // В подвале нельзя ставить кассу, стеллаж и лестницу
    if ecs.current_level == -1 {
        if active_slot.name == "cassa" || active_slot.name == "rack" || active_slot.name == "basement" {
            crate::audio::play("error");
            return;
        }
    }

    // Только один подвал на магазин
    if active_slot.name == "basement" && ecs.world.read_resource::<BasementPlaced>().0 {
        crate::audio::play("error");
        return;
    }

    // Мини-экономика: установка объекта стоит денег.
    let price = {
        let cfg = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>();
        super::object_price(active_slot.name, &cfg)
    };
    if ecs.world.read_resource::<Money>().0 < price {
        crate::audio::play("error");
        return;
    }

    if !ecs.can_place_at(
        gx, gy,
        active_slot.width, active_slot.height,
        is_carpet, is_light, is_wall_decor, is_outdoor, is_flower,
    ) {
        crate::audio::play("error");
        return;
    }

    ecs.clear_cursor_preview();

    ecs.world.write_resource::<Money>().0 -= price;
    crate::audio::play("place");

    // Создаём группу спрайтов размера width*height и получаем её id.
    let group_id = ecs.add_group_object(
        gx, gy,
        active_slot.width, active_slot.height,
        active_slot.path,
        active_slot.texture_frame,
        active_slot.texture_count,
        is_carpet,
        is_light,
        active_slot.animated,
        active_slot.frame_paths,
    );

    // Берём «главный» спрайт группы (первый) для привязки компонентов.
    let first = {
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
        groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
    };
    if let Some(entity) = first {
        // Запоминаем имя объекта и вешаем на него логику под конкретный тип.
        ecs.world.write_storage::<crate::ObjectTag>().insert(entity, crate::ObjectTag {
            name: active_slot.name.to_string(),
        }).ok();
        if active_slot.name == "basement" {
            // Подвал можно поставить только один — фиксируем это.
            ecs.world.write_resource::<BasementPlaced>().0 = true;
        } else if active_slot.name == "box" {
            // Ящик и стеллаж хранят еду для покупателей.
            let max_food = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().max_food_box;
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food,
            }).ok();
        } else if active_slot.name == "rack" {
            let max_food = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>().max_food_rack;
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: 0,
                max_food,
            }).ok();
        } else if active_slot.name == "candies" {
            // Конфеты начинаются с частично заполненного запаса.
            let cfg = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>();
            let (max_food, start) = (cfg.max_food_candies, cfg.candies_start_food);
            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                food_count: start,
                max_food,
            }).ok();
        } else if active_slot.name == "fence" || active_slot.name == "street_fence" {
            ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent { name: active_slot.name.to_string() }).ok();
        }
        if is_light_name(active_slot.name) || active_slot.name == "street_ice_cream" || active_slot.name == "arcade_machine" || active_slot.name == "candies" {
            attach_point_light(ecs, entity, active_slot.name);
        }
    }
}

// Убрать объект или вернуть часть стены/траву в клетке (gx, gy).
// Возвращает true, если что-то было удалено.
pub fn remove(ecs: &mut EcsAdapter, gx: i32, gy: i32) -> bool {
    // Сначала ищем объект (группу спрайтов) в клетке.
    if let Some(group_id) = ecs.find_group_at_position(gx, gy) {
        let entity = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = entity {
            // Если удаляем подвал — разрешаем поставить его снова.
            let is_basement = {
                let tags = ecs.world.read_storage::<crate::ObjectTag>();
                tags.get(entity).map(|t| t.name == "basement").unwrap_or(false)
            };
            if is_basement {
                ecs.world.write_resource::<BasementPlaced>().0 = false;
            }
            // Мини-экономика: возвращаем половину цены при удалении объекта.
            let name = {
                let tags = ecs.world.read_storage::<crate::ObjectTag>();
                tags.get(entity).map(|t| t.name.clone())
            };
            if let Some(name) = name {
                let cfg = ecs.world.read_resource::<crate::scripts::config::BalanceConfig>();
                let refund = super::object_price(&name, &cfg) / 2;
                ecs.world.write_resource::<Money>().0 += refund;
            }
        }
        ecs.delete_group(group_id);
        crate::audio::play("remove");
        return true;
    }
    // Если объекта нет — возможно, клетка является стеной.
    let file_col = gx + 21;
    let file_row = 14 - gy;
    if file_row >= 0 && file_row < ecs.map_grid.len() as i32 &&
       file_col >= 0 && file_col < ecs.map_grid[file_row as usize].len() as i32
    {
        let token = &ecs.map_grid[file_row as usize][file_col as usize];
        if token == "0" {
            // Пустая клетка — возвращаем траву и пересчитываем стены вокруг.
            revert_to_grass(ecs, gx, gy, file_row, file_col);
            refresh_walls_around(ecs, gx, gy);
            return true;
        }
        // Стена, выросшая из травы, тоже снимается в исходную траву.
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
