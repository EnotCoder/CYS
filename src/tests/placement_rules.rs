// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  placement_rules — can_place_at, установка (add) и снос (remove)
//  объектов: категории клеток, конфликты, экономика, подвал и стены.
// ========================================================================

use specs::WorldExt;

use crate::data::{add, get_slot_vec, make_slot, object_price, remove};
use crate::ecs::components::{
    BasementPlaced, FoodStorage, Money, ObjectTag, PlacementError, PointLight, ShopDenied, ShopOwned,
};
use crate::ecs::EcsAdapter;
use crate::scripts::config::BalanceConfig;
use crate::tests::helpers::{add_object, ecs_with_map, first_entity, group_count, new_ecs};

fn price_of(ecs: &EcsAdapter, name: &str) -> i32 {
    let cfg = ecs.world.read_resource::<BalanceConfig>();
    object_price(name, &cfg)
}

fn food_of(ecs: &EcsAdapter, entity: specs::Entity) -> Option<(i32, i32)> {
    let foods = ecs.world.read_storage::<FoodStorage>();
    foods.get(entity).map(|f| (f.food_count, f.max_food))
}

fn place(ecs: &mut EcsAdapter, name: &str, x: i32, y: i32) {
    let mut slots = get_slot_vec();
    slots[0] = make_slot(name);
    add(ecs, &mut slots, 0, x, y);
}

// ========================================================================
//  can_place_at — категории и конфликты
// ========================================================================

#[test]
fn can_place_at_enforces_category_cells() {
    let ecs = ecs_with_map();

    // (0, 0) — пол магазина, (0, 5) — стена, (-10, 2) — трава.
    assert!(ecs.floor_positions.contains(&(0, 0)));
    assert!(ecs.wall_positions.contains(&(0, 5)));
    assert!(ecs.outdoor_positions.contains(&(-10, 2)));
    assert!(ecs.flower_positions.contains(&(-10, 2)));

    assert!(ecs.can_place_at(0, 0, 1, 1, false, false, false, false, false));
    assert!(!ecs.can_place_at(0, 0, 1, 1, false, false, true, false, false));
    assert!(!ecs.can_place_at(0, 0, 1, 1, false, false, false, true, false));
    assert!(!ecs.can_place_at(0, 0, 1, 1, false, false, false, false, true));

    assert!(ecs.can_place_at(0, 5, 1, 1, false, false, true, false, false));
    assert!(!ecs.can_place_at(0, 5, 1, 1, false, false, false, false, false));

    assert!(ecs.can_place_at(-10, 2, 1, 1, false, false, false, true, false));
    assert!(ecs.can_place_at(-10, 2, 1, 1, false, false, false, false, true));
    assert!(!ecs.can_place_at(-10, 2, 1, 1, false, false, false, false, false));
}

#[test]
fn can_place_at_multicell_requires_all_cells() {
    let ecs = ecs_with_map();

    // 1x2 на (0,3) — обе клетки пол ((0,3) и (0,4)).
    assert!(ecs.can_place_at(0, 3, 1, 2, false, false, false, false, false));
    // 1x3 захватывает стену (0,5) — нельзя.
    assert!(!ecs.can_place_at(0, 3, 1, 3, false, false, false, false, false));
    // 2x1 на (5,0): (5,0) пол, (6,0) — граница пола 'D', тоже пол.
    assert!(ecs.can_place_at(5, 0, 2, 1, false, false, false, false, false));
    // 3x1 выходит за границу пола.
    assert!(!ecs.can_place_at(5, 0, 3, 1, false, false, false, false, false));
}

#[test]
fn can_place_at_rejects_same_category_overlap_only() {
    let mut ecs = ecs_with_map();

    add_object(&mut ecs, "blue_carpet", 0, 0);
    // Ковёр на ковёр нельзя, мебель и свет — можно.
    assert!(!ecs.can_place_at(0, 0, 1, 1, true, false, false, false, false));
    assert!(ecs.can_place_at(0, 0, 1, 1, false, false, false, false, false));
    assert!(ecs.can_place_at(0, 0, 1, 1, false, true, false, false, false));

    add_object(&mut ecs, "lamp", 1, 0);
    assert!(!ecs.can_place_at(1, 0, 1, 1, false, true, false, false, false));
    assert!(ecs.can_place_at(1, 0, 1, 1, false, false, false, false, false));
    assert!(ecs.can_place_at(1, 0, 1, 1, true, false, false, false, false));

    add_object(&mut ecs, "box", 2, 0);
    assert!(!ecs.can_place_at(2, 0, 1, 1, false, false, false, false, false));
    assert!(ecs.can_place_at(2, 0, 1, 1, true, false, false, false, false));
    assert!(ecs.can_place_at(2, 0, 1, 1, false, true, false, false, false));

    // Соседняя свободная клетка не считается занятой прямоугольником.
    assert!(ecs.can_place_at(3, 0, 1, 1, false, false, false, false, false));
}

// ========================================================================
//  add — экономика, доступ и компоненты
// ========================================================================

#[test]
fn add_charges_price_and_creates_one_group() {
    let mut ecs = ecs_with_map();
    let before = ecs.world.read_resource::<Money>().0;
    let price = price_of(&ecs, "box");
    assert!(before >= price);

    place(&mut ecs, "box", 0, 0);

    assert_eq!(ecs.world.read_resource::<Money>().0, before - price);
    assert_eq!(group_count(&ecs), 1);
    assert!(ecs.find_group_at_position(0, 0).is_some());

    let first = first_entity(&ecs, ecs.find_group_at_position(0, 0).unwrap());
    let foods = ecs.world.read_storage::<FoodStorage>();
    let food = foods.get(first).expect("нет FoodStorage");
    let cfg = ecs.world.read_resource::<BalanceConfig>();
    assert_eq!(food.food_count, 0);
    assert_eq!(food.max_food, cfg.max_food_box);
}

#[test]
fn add_rejects_unaffordable_placement_without_creating_object() {
    let mut ecs = ecs_with_map();
    ecs.world.write_resource::<Money>().0 = 5;
    let price = price_of(&ecs, "box");

    place(&mut ecs, "box", 0, 0);

    assert_eq!(group_count(&ecs), 0);
    assert_eq!(ecs.world.read_resource::<Money>().0, 5);
    assert_eq!(
        ecs.world.read_resource::<PlacementError>().0,
        Some((5, price))
    );
}

#[test]
fn add_rejects_unowned_shop_item() {
    let mut ecs = ecs_with_map();
    // candies не входит в SHOP_EXEMPT и ещё не куплен.
    assert!(!crate::data::placement::requires_shop("box"));
    assert!(crate::data::placement::requires_shop("candies"));

    place(&mut ecs, "candies", 0, 0);

    assert!(ecs.world.read_resource::<ShopDenied>().0);
    assert_eq!(group_count(&ecs), 0);

    // После покупки доступ разрешён, и объект создаётся.
    ecs.world.write_resource::<ShopDenied>().0 = false;
    ecs.world.write_resource::<ShopOwned>().0.push("candies".to_string());
    place(&mut ecs, "candies", 0, 0);
    assert!(!ecs.world.read_resource::<ShopDenied>().0);
    assert_eq!(group_count(&ecs), 1);
}

#[test]
fn add_allows_only_one_basement() {
    let mut ecs = ecs_with_map();
    ecs.world.write_resource::<ShopOwned>().0.push("basement".to_string());

    place(&mut ecs, "basement", 0, 0);
    assert!(ecs.world.read_resource::<BasementPlaced>().0);
    assert_eq!(group_count(&ecs), 1);

    place(&mut ecs, "basement", 2, 0);
    assert_eq!(group_count(&ecs), 1, "второй подвал не должен создаваться");
}

#[test]
fn add_rejects_cassa_rack_and_basement_in_the_basement() {
    for name in ["cassa", "rack", "basement"] {
        let mut ecs = ecs_with_map();
        ecs.current_level = -1;
        ecs.world.write_resource::<ShopOwned>().0.push(name.to_string());
        let before = ecs.world.read_resource::<Money>().0;

        place(&mut ecs, name, 0, 0);

        assert_eq!(group_count(&ecs), 0, "{name}: объект не должен ставиться в подвале");
        assert_eq!(ecs.world.read_resource::<Money>().0, before, "{name}: деньги списаны");
    }

    // box в подвале ставить можно.
    let mut ecs = ecs_with_map();
    ecs.current_level = -1;
    place(&mut ecs, "box", 0, 0);
    assert_eq!(group_count(&ecs), 1);
}

#[test]
fn add_initializes_food_and_lights() {
    let mut ecs = ecs_with_map();
    ecs.world.write_resource::<ShopOwned>().0.push("candies".to_string());
    ecs.world.write_resource::<ShopOwned>().0.push("lamp".to_string());
    let cfg_food = {
        let cfg = ecs.world.read_resource::<BalanceConfig>();
        (cfg.max_food_box, cfg.max_food_rack, cfg.max_food_candies, cfg.candies_start_food)
    };

    place(&mut ecs, "box", 0, 0);
    let box_gid = ecs.find_group_at_position(0, 0).unwrap();
    let box_entity = first_entity(&ecs, box_gid);
    let box_food = food_of(&ecs, box_entity).unwrap();
    assert_eq!(box_food, (0, cfg_food.0));

    place(&mut ecs, "rack", 2, 0);
    let rack_gid = ecs.find_group_at_position(2, 0).unwrap();
    let rack_entity = first_entity(&ecs, rack_gid);
    let rack_food = food_of(&ecs, rack_entity).unwrap();
    assert_eq!(rack_food, (0, cfg_food.1));

    place(&mut ecs, "candies", 4, 0);
    let candy_gid = ecs.find_group_at_position(4, 0).unwrap();
    let candy_entity = first_entity(&ecs, candy_gid);
    let candy_food = food_of(&ecs, candy_entity).unwrap();
    assert_eq!(candy_food, (cfg_food.3, cfg_food.2));
    assert!(
        ecs.world.read_storage::<PointLight>().get(candy_entity).is_some(),
        "у конфет должен быть PointLight"
    );

    place(&mut ecs, "lamp", 0, 1);
    let lamp_gid = ecs.find_group_at_position(0, 1).unwrap();
    let lamp_entity = first_entity(&ecs, lamp_gid);
    assert!(ecs.world.read_storage::<PointLight>().get(lamp_entity).is_some());
}

#[test]
fn remove_refunds_half_price_and_deletes_group() {
    let mut ecs = ecs_with_map();
    let price = price_of(&ecs, "box");
    place(&mut ecs, "box", 0, 0);
    let after_add = ecs.world.read_resource::<Money>().0;

    assert!(remove(&mut ecs, 0, 0));

    assert_eq!(ecs.world.read_resource::<Money>().0, after_add + price / 2);
    assert_eq!(group_count(&ecs), 0);
    assert!(ecs.find_group_at_position(0, 0).is_none());
    // Повторный снос клетки уже без объекта убирает сам пол (текущее поведение).
    assert!(remove(&mut ecs, 0, 0));
    assert!(!ecs.floor_positions.contains(&(0, 0)));
}

#[test]
fn remove_basement_frees_the_slot() {
    let mut ecs = ecs_with_map();
    ecs.world.write_resource::<ShopOwned>().0.push("basement".to_string());
    place(&mut ecs, "basement", 0, 0);
    assert!(ecs.world.read_resource::<BasementPlaced>().0);
    let price = price_of(&ecs, "basement");
    let before = ecs.world.read_resource::<Money>().0;

    assert!(remove(&mut ecs, 0, 0));

    assert!(!ecs.world.read_resource::<BasementPlaced>().0);
    assert_eq!(ecs.world.read_resource::<Money>().0, before + price / 2);
}

// ========================================================================
//  remove — пересчёт стен (синтетическая карта)
// ========================================================================

/// Мир с одной строкой сетки на y = 0: колонки 26/27/28 = gx 0/1/2.
fn synthetic_ecs(cells: &[(i32, &str)], originals: &[(i32, &str)]) -> EcsAdapter {
    let mut ecs = new_ecs();
    let mut grid = vec![vec![".".to_string(); 30]; 15];
    for &(gx, token) in cells {
        grid[14][(gx + 26) as usize] = token.to_string();
    }
    ecs.map_grid = grid;
    for &(gx, token) in originals {
        ecs.original_tokens.insert((gx, 0), token.to_string());
    }
    for &(gx, token) in cells {
        if token == "0" {
            ecs.floor_positions.insert((gx, 0));
        }
    }
    ecs
}

#[test]
fn remove_floor_refreshes_generated_walls_to_grass() {
    // (0,0) — пол, (1,0) — сгенерированная стена "&"; исходно обе — трава.
    let mut ecs = synthetic_ecs(
        &[(0, "0"), (1, "&")],
        &[(0, "."), (1, ".")],
    );

    assert!(remove(&mut ecs, 0, 0));

    assert_eq!(ecs.map_grid[14][26], ".", "пол должен вернуться в траву");
    assert_eq!(ecs.map_grid[14][27], ".", "стена без пола рядом должна исчезнуть");
    assert!(!ecs.floor_positions.contains(&(0, 0)));
    assert!(ecs.outdoor_positions.contains(&(0, 0)));
}

#[test]
fn remove_floor_keeps_static_walls_and_door() {
    // (1,0) — статическая стена "W", (2,0) — дверной проём "E".
    let mut ecs = synthetic_ecs(
        &[(0, "0"), (1, "W"), (2, "E")],
        &[(0, "."), (1, "W"), (2, "E")],
    );

    assert!(remove(&mut ecs, 0, 0));

    assert_eq!(ecs.map_grid[14][26], ".");
    assert_eq!(ecs.map_grid[14][27], "W", "статическая стена не должна меняться");
    assert_eq!(ecs.map_grid[14][28], "E", "дверной проём не должен меняться");

    // Статическую стену нельзя «снести» как траву.
    assert!(!remove(&mut ecs, 1, 0));
    assert_eq!(ecs.map_grid[14][27], "W");
}

#[test]
fn remove_object_does_not_touch_map_cell_below() {
    let mut ecs = ecs_with_map();
    let before_token = ecs.map_grid[14][26].clone();
    add_object(&mut ecs, "box", 0, 0);

    // remove находит группу раньше карты и не должен менять токен пола.
    assert!(remove(&mut ecs, 0, 0));
    assert_eq!(ecs.map_grid[14][26], before_token);
    assert!(ecs.floor_positions.contains(&(0, 0)));
}

#[test]
fn group_metadata_survives_until_removed() {
    let mut ecs = ecs_with_map();
    add_object(&mut ecs, "box", 0, 0);
    let gid = ecs.find_group_at_position(0, 0).unwrap();
    {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
        let group = groups.groups.get(&gid).unwrap();
        assert_eq!(group.name, "box");
        assert!(group.entities.iter().all(|e| tags.get(*e).is_some()));
    }

    ecs.delete_group(gid);
    assert_eq!(group_count(&ecs), 0);
    assert!(ecs.find_group_at_position(0, 0).is_none());
}
