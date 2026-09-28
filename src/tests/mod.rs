// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  tests — модуль юнит-тестов крейта.
//  Подключается из src/main.rs директивой #[cfg(test)] mod tests;
//  сюда добавляются тесты по мере развития игры.
// ========================================================================

#[test]
fn mode_cycle_includes_move_mode() {
    use crate::core::constants::{MODE_BUILD, MODE_DELETE, MODE_INTERACT, MODE_MOVE};

    assert_eq!(crate::input::interact::next_mode(MODE_MOVE), MODE_INTERACT);
    assert_eq!(crate::input::interact::next_mode(MODE_INTERACT), MODE_BUILD);
    assert_eq!(crate::input::interact::next_mode(MODE_BUILD), MODE_DELETE);
    assert_eq!(crate::input::interact::next_mode(MODE_DELETE), MODE_MOVE);
}

#[test]
fn text_input_accepts_unicode_and_focus_request() {
    let input = crate::ui::text_input::TextInput::new();
    input.request_focus();
    assert!(input.is_active());
    assert!(input.take_focus_request());
    assert!(!input.take_focus_request());

    input.push("Привет, мир");
    assert_eq!(input.take(), "Привет, мир");

    input.set_active(false);
    assert!(!input.is_active());
}

// ========================================================================
//  Дверь магазина и проходимость
// ========================================================================

#[test]
fn door_opening_is_four_cells_in_the_south_facade() {
    use crate::core::constants::{DOOR_X_MAX, DOOR_X_MIN, DOOR_Y_MAX, DOOR_Y_MIN, SIDEWALK_Y};
    use crate::data::map::{door_cells, is_door_token};

    let cells = door_cells();
    assert_eq!(cells.len(), 4, "проём должен быть 2x2");
    for x in DOOR_X_MIN..=DOOR_X_MAX {
        for y in DOOR_Y_MIN..=DOOR_Y_MAX {
            assert!(cells.contains(&(x, y)), "нет клетки двери ({x},{y})");
        }
    }
    // Дверь должна стоять вплотную к тротуару, иначе покупатель не дойдёт.
    assert_eq!(DOOR_Y_MIN, SIDEWALK_Y + 1, "нижний край двери должен примыкать к тротуару");
    assert!(is_door_token(crate::core::constants::DOOR_TOKEN));
    // Дверь — не пол и не стена: на неё ничего нельзя поставить, но она
    // не должна попадать в пересчёт стен.
    assert!(!crate::data::map::is_floor_tile(crate::core::constants::DOOR_TOKEN));
    assert!(!crate::data::map::is_wall_tile(crate::core::constants::DOOR_TOKEN));
    assert!(!crate::data::is_grass_token(crate::core::constants::DOOR_TOKEN));
}

#[test]
fn door_texture_steps_cover_the_animation_cycle() {
    use crate::core::constants::{
        DOOR_TEX_CLOSED, DOOR_TEX_OPEN, DOOR_TEX_OPEN_1, DOOR_TEX_OPEN_2, DOOR_TEX_OPEN_3,
    };
    use crate::data::map::door_texture_for_step;

    // 0 — закрыта, 1..3 — распахивание, дальше полностью открыта.
    assert_eq!(door_texture_for_step(0), DOOR_TEX_CLOSED);
    assert_eq!(door_texture_for_step(-1), DOOR_TEX_CLOSED, "отрицательный шаг тоже закрытая");
    assert_eq!(door_texture_for_step(1), DOOR_TEX_OPEN_1);
    assert_eq!(door_texture_for_step(2), DOOR_TEX_OPEN_2);
    assert_eq!(door_texture_for_step(3), DOOR_TEX_OPEN_3);
    assert_eq!(door_texture_for_step(4), DOOR_TEX_OPEN, "дальше открыта");

    // Все пять текстур должны быть разными файлами.
    let all = [DOOR_TEX_CLOSED, DOOR_TEX_OPEN_1, DOOR_TEX_OPEN_2, DOOR_TEX_OPEN_3, DOOR_TEX_OPEN];
    for i in 0..all.len() {
        assert!(crate::core::asset::load_bytes(all[i]).is_ok(), "нет текстуры {}", all[i]);
        for j in i + 1..all.len() {
            assert_ne!(all[i], all[j]);
        }
    }
}

#[test]
fn shopper_can_walk_from_the_street_into_the_shop_through_the_door() {
    use crate::data::map::{door_cells, load_walkable_cells, shopper_spawn_point};
    use crate::data::map::pathfinding::find_path;

    let walkable = load_walkable_cells();

    // Точка входа с улицы и обе клетки пола должны быть проходимы.
    let spawn = shopper_spawn_point();
    assert!(walkable.contains(&spawn), "точка входа с улицы не проходима: {spawn:?}");

    // Проём целиком проходим (иначе покупатель не пройдёт).
    for (x, y) in door_cells() {
        let cell = crate::data::map::pathfinding::Node::new(x, y);
        assert!(walkable.contains(&cell), "клетка двери ({x},{y}) не проходима");
    }

    // Ключевая проверка: покупатель с улицы доходит до пола магазина.
    // Раньше магазин был изолирован (77 клеток), и пути не существовало.
    let inside = crate::data::map::pathfinding::Node::new(0, -3);
    assert!(walkable.contains(&inside), "пол магазина не проходим");
    let path = find_path(&walkable, spawn, inside)
        .unwrap_or_else(|| panic!("нет пути с улицы {spawn:?} в магазин {inside:?}"));
    assert!(path.len() > 2, "маршрут подозрительно короткий: {} клеток", path.len());

    // Маршрут обязан проходить через дверь, а не в обход неё.
    assert!(
        path.iter().any(|n| door_cells().iter().any(|(x, y)| *x == n.x && *y == n.y)),
        "покупатель не проходит через дверь"
    );

    // Обратный маршрут (выход) тоже должен существовать.
    assert!(find_path(&walkable, inside, spawn).is_some(), "покупатель не может выйти на улицу");
}

#[test]
fn shopper_walks_in_through_the_door_and_back_out() {
    use specs::WorldExt;
    use crate::core::constants::*;
    use crate::data::map::{door_cells, load_map_to_ecs, load_walkable_cells, shopper_spawn_point};
    use crate::ecs::components::{BusyCassas, FoodStorage, ObjectTag};
    use crate::npc::ShopperNpc;

    let mut ecs = crate::EcsAdapter::new();
    load_map_to_ecs(&mut ecs);
    let walkable = load_walkable_cells();

    // Стеллаж с едой и касса внутри магазина.
    let rack_gid = ecs.add_group_object(
        0, 2, 1, 2, "rack", "assets/tex/decor/regular/rack/rack_0.png",
        [0, 1], [1, 2], false, false, false, &[],
    );
    let cassa_gid = ecs.add_group_object(
        2, -2, 2, 2, "cassa", "assets/tex/decor/regular/cassa.png",
        [0, 1], [2, 2], false, false, false, &[],
    );
    {
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>().clone();
        let rack_ent = groups.groups.get(&rack_gid).unwrap().entities[0];
        let cassa_ent = groups.groups.get(&cassa_gid).unwrap().entities[0];
        ecs.world.write_storage::<ObjectTag>().insert(rack_ent, ObjectTag { name: "rack".into() }).unwrap();
        ecs.world.write_storage::<ObjectTag>().insert(cassa_ent, ObjectTag { name: "cassa".into() }).unwrap();
        ecs.world.write_storage::<FoodStorage>().insert(rack_ent, FoodStorage { food_count: 10, max_food: 15 }).unwrap();
    }
    ecs.world.write_resource::<BusyCassas>().0.clear();

    // Клетка перед стеллажом, куда подходит покупатель (как в ShopperManager).
    let rack_pos = crate::data::map::pathfinding::Node::new(0, 3);
    let cassa_pos = crate::data::map::pathfinding::Node::new(2, -2);
    let spawn = shopper_spawn_point();

    let mut shopper = ShopperNpc::spawn(
        &mut ecs, &walkable, spawn, rack_pos, cassa_pos, None,
        TEX_BOB_IDLE, TEX_BOB_WALK_1, TEX_BOB_WALK_2,
    ).expect("покупатель не заспавнился: нет пути с улицы");

    // Пока покупатель далеко, дверь закрыта.
    crate::data::door::tick_door(&mut ecs, 1.0 / 60.0);
    assert_eq!(ecs.world.read_resource::<crate::ecs::components::DoorProgress>().state,
               crate::ecs::components::DoorState::Closed);

    let dt = 1.0 / 60.0;
    let mut door_opened = false;
    let mut went_through_door = false;
    let mut bought = false;
    let mut left_through_door = false;
    let mut despawned = false;

    for _ in 0..(60 * 90) {
        let before = shopper.pos();
        let done = shopper.update(&mut ecs, dt, &walkable, None);
        let after = shopper.pos();
        // Дверь тикаем после шага NPC, чтобы видеть его актуальную позицию.
        crate::data::door::tick_door(&mut ecs, dt);

        let state = ecs.world.read_resource::<crate::ecs::components::DoorProgress>().state;
        if state == crate::ecs::components::DoorState::Open {
            door_opened = true;
        }
        // Граница проёма — между внутренним паркетом (Y = -4) и фасадом.
        if before.1 < -4.5 && after.1 >= -4.5 {
            went_through_door = true;
        }
        if before.1 >= -4.5 && after.1 < -4.5 {
            left_through_door = true;
        }
        if shopper.has_taken_food() {
            bought = true;
        }
        if done {
            despawned = true;
            break;
        }
    }

    assert!(door_opened, "дверь так и не открылась");
    assert!(went_through_door, "покупатель не вошёл в магазин через дверь");
    assert!(bought, "покупатель не взял товар");
    assert!(left_through_door, "покупатель не вышел обратно через дверь");
    assert!(despawned, "покупатель не ушёл");

    // Покупатель у проёма плавно гаснет и проявляется, а не «щёлкает».
    // Проверяем на живом списке спрайтов: внутри проёма альфа 0 (спрайт вообще
    // не отправляется в батчет), снаружи — 1, а в промежутке обязаны быть
    // промежуточные значения, иначе на границе клеток будет щелчок.
    {
        let npc_alpha = |ecs: &crate::EcsAdapter| -> Option<f32> {
            ecs.get_sprites_by_layer(None).4.first().map(|s| s.alpha)
        };
        // Покупатель к этому моменту деспавнился, и его собственная альфа
        // затухания равна 0. Затухание у проёма домножается на неё, поэтому
        // для проверки рампы возвращаем альфу в 1.
        ecs.update_sprite_alpha(shopper.entity, 1.0);
        // Внутри проёма: спрайта нет вовсе.
        for (gx, gy) in [(-1, -6), (0, -6), (-1, -5), (0, -5)] {
            ecs.update_transform_position(shopper.entity, gx as f32, gy as f32);
            assert!(npc_alpha(&ecs).is_none(),
                    "в клетке проёма ({gx},{gy}) покупатель не должен рисоваться");
        }
        // Середина рампы: ровно 0.5 альфы (это 0.25 клетки от края проёма).
        for gy in [-6.75, -4.25] {
            ecs.update_transform_position(shopper.entity, 0.0, gy);
            let a = npc_alpha(&ecs).expect("на рампе покупатель должен рисоваться");
            assert!((a - 0.5).abs() < 0.02,
                    "на середине рампы (y={gy}) ожидалась альфа 0.5, а {a}");
        }
        // Границы рампы: у края тротуара и у края паркета — полная видимость.
        for gy in [-7.0, -4.0] {
            ecs.update_transform_position(shopper.entity, 0.0, gy);
            let a = npc_alpha(&ecs).expect("покупатель обязан рисоваться");
            assert!((a - 1.0).abs() < 0.001,
                    "на границе (y={gy}) ожидалась альфа 1.0, а {a}");
        }
        // Рампа обязана быть непрерывной: ни одного скачка между 0 и 1.
        let mut prev = 1.0f32;
        let mut monotonic_down = true;
        let mut monotonic_up = true;
        for i in 0..=40 {
            let y = -7.0 + i as f32 * (3.0 / 40.0);
            ecs.update_transform_position(shopper.entity, 0.0, y);
            let a = npc_alpha(&ecs).unwrap_or(0.0);
            if y < -5.0 {
                if a > prev + 0.001 { monotonic_down = false; }
            } else {
                if a < prev - 0.001 { monotonic_up = false; }
            }
            prev = a;
        }
        assert!(monotonic_down, "альфа должна только падать на входе");
        assert!(monotonic_up, "альфа должна только расти на выходе");
    }

    // Дверь закрылась после ухода покупателя.
    for _ in 0..(60 * 5) {
        crate::data::door::tick_door(&mut ecs, dt);
    }
    assert_eq!(ecs.world.read_resource::<crate::ecs::components::DoorProgress>().state,
               crate::ecs::components::DoorState::Closed, "дверь не закрылась после ухода покупателя");

    // Проверяем, что клетки двери ссылаются на 2x2 атлас и у каждой СВОЙ кадр.
    // Раньше все четыре получали кадр [0,0] и проём собирался из четырёх
    // одинаковых углов — этот тест ловит именно такую ошибку.
    let mut frames: Vec<[i32; 2]> = Vec::new();
    for (x, y) in door_cells() {
        let entity = ecs.map_entities[&(x, y)];
        let sprites = ecs.world.read_storage::<crate::ecs::components::SpriteComponent>();
        let s = sprites.get(entity).expect("нет спрайта двери");
        assert_eq!(s.texture_count, [2, 2], "дверь должна быть 2x2 атласом");
        assert_eq!(s.texture_frame, crate::data::map::door_frame_for_cell(x, y),
                   "неверный кадр клетки ({x},{y})");
        assert!(s.texture_path.as_ref().ends_with("shop_door.png"),
                "дверь должна ссылаться на текстуру двери, а не на {}",
                s.texture_path);
        frames.push(s.texture_frame);
    }
    frames.sort_unstable();
    assert_eq!(frames, vec![[0, 0], [0, 1], [1, 0], [1, 1]],
               "у четырёх клеток двери должны быть четыре разных кадра атласа");

    // Дверь переживает пересборку мира и не теряет ни кадр, ни слой.
    // Проверяем оба пути создания тайлов — обычную загрузку карты и
    // восстановление уровня из кэша (общий spawn_map_tile). Раньше логика
    // двери жила только в загрузке, и после похода в подвал клетки
    // пересоздавались как обычные тайлы на Z_MAP — дверь уезжала под NPC.
    {
        let mut ecs = crate::EcsAdapter::new();
        // Путь 1: восстановление уровня, как это делает GameScene::load_level.
        for (x, y) in door_cells() {
            crate::data::map::spawn_map_tile(&mut ecs, DOOR_TOKEN, x as f32, y as f32, x, y);
        }
        let transforms = ecs.world.read_storage::<crate::ecs::components::Transform>();
        let sprites = ecs.world.read_storage::<crate::ecs::components::SpriteComponent>();
        for (x, y) in door_cells() {
            let entity = ecs.map_entities[&(x, y)];
            let t = transforms.get(entity).expect("нет клетки двери после spawn_map_tile");
            assert!((t.position[2] - Z_DOOR).abs() < 0.001,
                    "через spawn_map_tile клетка ({x},{y}) уехала на слой {}", t.position[2]);
            let s = sprites.get(entity).unwrap();
            assert_eq!(s.texture_frame, crate::data::map::door_frame_for_cell(x, y),
                       "через spawn_map_tile у клетки ({x},{y}) сбился кадр");
        }
        // Путь 2: полная перезагрузка карты в чистом мире.
        let mut ecs2 = crate::EcsAdapter::new();
        load_map_to_ecs(&mut ecs2);
        ecs2.clear_world();
        load_map_to_ecs(&mut ecs2);
        let transforms2 = ecs2.world.read_storage::<crate::ecs::components::Transform>();
        for (x, y) in door_cells() {
            let entity = ecs2.map_entities[&(x, y)];
            let t = transforms2.get(entity).expect("после перезагрузки карты нет двери");
            assert!((t.position[2] - Z_DOOR).abs() < 0.001,
                    "после перезагрузки карты клетка ({x},{y}) уехала на слой {}", t.position[2]);
        }
    }

    // Дверь рисуется ПОД покупателем: створка не должна закрывать человека,
    // который стоит перед магазином. При этом слой обязан остаться в
    // ПРОЗРАЧНОМ проходе (Z_DECOR, а не Z_MAP) — иначе альфа проёма
    // открытой двери проигнорируется и проём выйдет чёрным.
    assert!(Z_DOOR < Z_NPC, "дверь должна быть ниже NPC");
    assert!(Z_DOOR >= Z_MAP, "дверь не должна проваливаться под карту");
    assert_eq!(Z_DOOR, Z_DECOR, "дверь обязана быть в прозрачном проходе");
    let transforms = ecs.world.read_storage::<crate::ecs::components::Transform>();
    for (x, y) in door_cells() {
        let entity = ecs.map_entities[&(x, y)];
        let t = transforms.get(entity).expect("нет трансформа у клетки двери");
        assert!((t.position[2] - Z_DOOR).abs() < 0.001,
                "клетка двери ({x},{y}) не на слое Z_DOOR, а на {}", t.position[2]);
    }
}
