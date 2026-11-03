// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  npc_tests — покупатель: автомат состояний, движение, еда/конфеты,
//  кассы, маршрутизация и фейд-аут.
// ========================================================================

use specs::WorldExt;

use crate::core::constants::{TEX_BOB_IDLE, TEX_BOB_WALK_1, TEX_BOB_WALK_2};
use crate::data::map::pathfinding::Node;
use crate::data::map::{load_walkable_cells, shopper_spawn_point};
use crate::ecs::components::{BusyCassas, FoodStorage, Money, Transform};
use crate::npc::ShopperNpc;
use crate::scripts::config::BalanceConfig;
use crate::tests::helpers::{add_object, ecs_with_map, first_entity, set_food};
use crate::EcsAdapter;

fn spawn(
    ecs: &mut EcsAdapter,
    rack: Node,
    cassa: Node,
    candy: Option<Node>,
) -> ShopperNpc {
    let walkable = load_walkable_cells();
    ShopperNpc::spawn(
        ecs,
        &walkable,
        shopper_spawn_point(),
        rack,
        cassa,
        candy,
        TEX_BOB_IDLE,
        TEX_BOB_WALK_1,
        TEX_BOB_WALK_2,
    )
    .expect("нет пути от входа до стеллажа")
}

fn food_of(ecs: &EcsAdapter, entity: specs::Entity) -> Option<(i32, i32)> {
    ecs.world
        .read_storage::<FoodStorage>()
        .get(entity)
        .map(|f| (f.food_count, f.max_food))
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[test]
fn state_int_roundtrips_for_all_states() {
    let mut ecs = ecs_with_map();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);

    for state in 1..=6 {
        npc.set_state_int(state);
        assert_eq!(npc.state_int(), state);
    }
    // Неизвестное значение сбрасывает в «идёт к стеллажу».
    npc.set_state_int(99);
    assert_eq!(npc.state_int(), 1);
    npc.set_state_int(-1);
    assert_eq!(npc.state_int(), 1);
}

#[test]
fn take_food_takes_one_item_per_call() {
    let mut ecs = ecs_with_map();
    let gid = add_object(&mut ecs, "rack", -1, -3);
    set_food(&mut ecs, gid, 2, 15);
    let rack_entity = first_entity(&ecs, gid);
    let mut npc = spawn(&mut ecs, Node::new(-1, -2), Node::new(2, -3), None);

    assert_eq!(food_of(&ecs, rack_entity), Some((2, 15)));
    assert!(npc.take_food(&mut ecs));
    assert_eq!(food_of(&ecs, rack_entity), Some((1, 15)));
    assert!(npc.has_taken_food());
    assert!(npc.is_food_taken());

    assert!(npc.take_food(&mut ecs));
    assert_eq!(food_of(&ecs, rack_entity), Some((0, 15)));
    assert!(!npc.take_food(&mut ecs), "пустой стеллаж не отдаёт еду");
    assert_eq!(food_of(&ecs, rack_entity), Some((0, 15)));
}

#[test]
fn take_food_accepts_the_lower_cell_of_a_two_tall_rack() {
    let mut ecs = ecs_with_map();
    let gid = add_object(&mut ecs, "rack", 0, -4);
    // Еда лежит на верхней клетке (y = -4), а покупатель стоит на нижней (y = -3).
    let upper = first_entity(&ecs, gid);
    ecs.world
        .write_storage::<FoodStorage>()
        .insert(upper, FoodStorage { food_count: 1, max_food: 15 })
        .ok();

    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    assert!(npc.take_food(&mut ecs));
    assert_eq!(food_of(&ecs, upper), Some((0, 15)));
}

#[test]
fn take_candy_requires_a_position_and_food() {
    let mut ecs = ecs_with_map();
    let gid = add_object(&mut ecs, "candies", 1, -3);
    set_food(&mut ecs, gid, 1, 10);
    let candy_entity = first_entity(&ecs, gid);

    let mut npc = spawn(
        &mut ecs,
        Node::new(0, -3),
        Node::new(2, -3),
        Some(Node::new(1, -3)),
    );
    assert!(npc.candy_exists(&ecs));
    assert!(npc.take_candy(&mut ecs));
    assert_eq!(food_of(&ecs, candy_entity), Some((0, 10)));
    assert!(!npc.candy_exists(&ecs));
    assert!(!npc.take_candy(&mut ecs));

    // Без candy_pos конфет нет, даже если объект стоит.
    set_food(&mut ecs, gid, 5, 10);
    let mut no_candy = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    assert!(!no_candy.candy_exists(&ecs));
    assert!(!no_candy.take_candy(&mut ecs));
}

#[test]
fn cassa_exists_checks_the_exact_cell() {
    let mut ecs = ecs_with_map();
    let npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    assert!(!npc.cassa_exists(&ecs));
    assert!(ShopperNpc::find_any_cassa(&ecs).is_none());

    add_object(&mut ecs, "cassa", 2, -3);
    assert!(npc.cassa_exists(&ecs));
    assert_eq!(ShopperNpc::find_any_cassa(&ecs), Some(Node::new(2, -3)));

    let no_cassa = spawn(&mut ecs, Node::new(0, -3), Node::new(5, 0), None);
    assert!(!no_cassa.cassa_exists(&ecs), "касса на другой клетке не считается");
}

#[test]
fn reroute_to_cassa_moves_the_busy_marker_and_state() {
    let mut ecs = ecs_with_map();
    add_object(&mut ecs, "cassa", 2, -3);
    add_object(&mut ecs, "cassa", 4, -3);
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    ecs.world
        .write_resource::<BusyCassas>()
        .0
        .insert((2, -3));

    let walkable = load_walkable_cells();
    npc.reroute_to_cassa(&mut ecs, &walkable, Node::new(4, -3));

    assert_eq!(npc.cassa_pos(), Node::new(4, -3));
    assert_eq!(npc.state_int(), 2);
    let busy = &ecs.world.read_resource::<BusyCassas>().0;
    assert!(busy.contains(&(4, -3)));
    assert!(!busy.contains(&(2, -3)));
}

#[test]
fn walk_toward_never_moves_faster_than_speed_per_frame() {
    let mut ecs = ecs_with_map();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    let speed = ecs.world.read_resource::<BalanceConfig>().npc_speed;

    for dt in [0.001, 0.01, 0.05] {
        let before = npc.pos();
        npc.walk_toward(&mut ecs, dt);
        let moved = dist(before, npc.pos());
        assert!(
            moved <= speed * dt as f32 + 1e-4,
            "шаг {moved} больше {speed}*{dt}"
        );
    }
}

#[test]
fn walk_toward_reaches_the_rack_and_updates_the_sprite() {
    let mut ecs = ecs_with_map();
    let rack = Node::new(0, -3);
    let mut npc = spawn(&mut ecs, rack, Node::new(2, -3), None);

    for _ in 0..4000 {
        if npc.path_done() {
            break;
        }
        npc.walk_toward(&mut ecs, 0.05);
    }

    assert!(npc.path_done(), "покупатель не дошёл до конца пути");
    let pos = npc.pos();
    assert!(dist(pos, (rack.x as f32, rack.y as f32)) < 1e-4);
    let transforms = ecs.world.read_storage::<Transform>();
    let t = transforms.get(npc.entity).unwrap();
    assert!((t.position[0] - pos.0).abs() < 1e-4);
    assert!((t.position[1] - pos.1).abs() < 1e-4);
}

#[test]
fn walk_to_exit_returns_true_only_at_the_target() {
    let mut ecs = ecs_with_map();
    let walkable = load_walkable_cells();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);

    assert!(npc.start_path(&ecs, &walkable, Node::new(14, -7)));
    let mut done = false;
    for _ in 0..4000 {
        done = npc.walk_to_exit(&mut ecs, 0.05);
        if done {
            break;
        }
    }

    assert!(done, "не дошёл до выхода");
    let pos = npc.pos();
    assert!((pos.0 - 14.0).abs() < 1e-4);
    assert!((pos.1 - (-6.5)).abs() < 1e-4, "позиция выхода: {pos:?}");
}

#[test]
fn update_fades_in_then_out_before_despawn() {
    let mut ecs = ecs_with_map();
    let walkable = load_walkable_cells();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    let fade = ecs.world.read_resource::<BalanceConfig>().npc_fade_speed;
    let dt = 0.25;

    // Появление: alpha 0 -> 1 за 1/fade секунд.
    assert!(!npc.update(&mut ecs, dt, &walkable, None));
    assert!(!npc.update(&mut ecs, dt, &walkable, None));

    npc.request_despawn();
    // Исчезновение: нужно ещё 1/fade секунд, затем update вернёт true.
    let mut ticks = 0;
    let mut despawned = false;
    while ticks < 100 {
        despawned = npc.update(&mut ecs, dt as f64, &walkable, None);
        ticks += 1;
        if despawned {
            break;
        }
    }
    assert!(despawned, "покупатель не деспавнится после фейда (fade={fade})");
    assert!(ticks >= 2);
    let alpha = ecs
        .world
        .read_storage::<crate::ecs::components::SpriteComponent>()
        .get(npc.entity)
        .unwrap()
        .alpha;
    assert_eq!(alpha, 0.0);
}

#[test]
fn no_cassa_sends_the_shopper_to_the_exit() {
    let mut ecs = ecs_with_map();
    let walkable = load_walkable_cells();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);

    assert!(!npc.update(&mut ecs, 0.1, &walkable, None));

    assert_eq!(npc.state_int(), 6, "без кассы покупатель должен уходить");
}

#[test]
fn exiting_toggles_between_exit_and_rack() {
    let mut ecs = ecs_with_map();
    let rack_gid = add_object(&mut ecs, "rack", 0, -3);
    set_food(&mut ecs, rack_gid, 5, 15);
    add_object(&mut ecs, "cassa", 2, -3);
    let walkable = load_walkable_cells();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);

    npc.set_exiting(true);
    assert!(!npc.update(&mut ecs, 0.05, &walkable, None));
    assert_eq!(npc.state_int(), 6);

    npc.set_exiting(false);
    assert!(!npc.update(&mut ecs, 0.05, &walkable, None));
    assert_eq!(npc.state_int(), 1, "active=true возвращает к стеллажу");
}

#[test]
fn paid_cassa_increases_money_and_releases_busy_marker() {
    let mut ecs = ecs_with_map();
    add_object(&mut ecs, "rack", 0, -3);
    add_object(&mut ecs, "cassa", 2, -3);
    let cfg = ecs.world.read_resource::<BalanceConfig>();
    let wait = cfg.cassa_wait_secs;
    let income = cfg.money_at_cassa;
    drop(cfg);

    let walkable = load_walkable_cells();
    let mut npc = spawn(&mut ecs, Node::new(0, -3), Node::new(2, -3), None);
    // Покупатель уже стоит у кассы с едой и занятой клеткой.
    npc.set_state_int(3);
    npc.set_state_timer(wait);
    npc.set_exiting(false);
    ecs.world.write_resource::<BusyCassas>().0.insert((2, -3));
    let money_before = ecs.world.read_resource::<Money>().0;

    assert!(!npc.update(&mut ecs, wait * 2.0, &walkable, None));

    assert_eq!(ecs.world.read_resource::<Money>().0, money_before + income);
    assert!(!ecs.world.read_resource::<BusyCassas>().0.contains(&(2, -3)));
}
