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

// ========================================================================
//  Поиск пути (A*)
// ========================================================================

// Прямоугольная сетка [-w..=w] x [-h..=h] целиком из проходимых клеток.
#[cfg(test)]
fn walkable_rect(w: i32, h: i32) -> std::collections::HashSet<crate::data::map::pathfinding::Node> {
    use crate::data::map::pathfinding::Node;
    let mut set = std::collections::HashSet::new();
    for x in -w..=w {
        for y in -h..=h {
            set.insert(Node::new(x, y));
        }
    }
    set
}

#[test]
fn path_on_open_field_is_optimal_and_walkable() {
    use crate::data::map::pathfinding::{find_path, Node};

    let walkable = walkable_rect(12, 12);
    let start = Node::new(0, 0);
    let goal = Node::new(9, 3);
    let path = find_path(&walkable, start, goal).expect("на пустом поле путь обязан существовать");

    // На открытом поле без препятствий кратчайший путь равен манхэттенскому
    // расстоянию плюс стартовая клетка. Эвристика find_path — манхэттенская,
    // поэтому если она когда-то станет неточной (например, с диагоналями или
    // разными весами рёбер), тест это поймает.
    let manhattan = (start.x - goal.x).abs() + (start.y - goal.y).abs();
    assert_eq!(path.len(), manhattan as usize + 1,
               "путь должен быть кратчайшим, а не каким-то обходом");

    // Путь начинается в старте, кончается в цели, идёт только по проходимым
    // клеткам и каждым шагом смещается ровно на одну клетку по оси
    // (движение строго 4-связное, диагоналей быть не должно).
    assert_eq!(path[0], start, "маршрут должен начинаться в старте");
    assert_eq!(*path.last().unwrap(), goal, "маршрут должен кончаться в цели");
    for (i, node) in path.iter().enumerate() {
        assert!(walkable.contains(node), "клетка {node:?} вне проходимой области");
        if i > 0 {
            let prev = path[i - 1];
            let step = (prev.x - node.x).abs() + (prev.y - node.y).abs();
            assert_eq!(step, 1,
                       "переход {prev:?} -> {node:?} должен быть шагом в одну клетку, а не скачком");
        }
    }
}

#[test]
fn path_reports_unreachable_and_out_of_bounds() {
    use crate::data::map::pathfinding::{find_path, Node};

    let mut walkable = walkable_rect(4, 4);

    // Старт равен цели — путь состоит из одной клетки, а не пустой.
    assert_eq!(find_path(&walkable, Node::new(2, 2), Node::new(2, 2)),
               Some(vec![Node::new(2, 2)]));

    // Старт или цель вне проходимой области — пути нет.
    assert_eq!(find_path(&walkable, Node::new(-9, 0), Node::new(1, 1)), None,
               "старт вне карты должен давать None, а не путь по воздуху");
    assert_eq!(find_path(&walkable, Node::new(1, 1), Node::new(0, 9)), None);

    // Цель заперта в отдельной клетке-островке: до неё физически не дойти.
    let island = Node::new(20, 20);
    walkable.insert(island);
    assert_eq!(find_path(&walkable, Node::new(0, 0), island), None,
               "изолированная клетка недостижима, A* обязан честно вернуть None");
    assert_eq!(find_path(&walkable, island, Node::new(0, 0)), None,
               "и обратно из неё тоже");
}

#[test]
fn world_to_cell_snapping_survives_round_trip() {
    use crate::data::map::pathfinding::Node;

    // from_world округляет «половинки вверх» ((v + 0.5).floor()), поэтому
    // отрицательные координаты — отдельный случай со знаками. to_world
    // возвращает целое (x, y), то есть левый-нижний угол клетки.
    // Округление round-half-up: ровно половина клетки уходит ВВЕРХ, в том
    // числе для отрицательных координат (-0.5 -> 0, а не -1).
    let cases: &[(f32, i32)] = &[(-1.6, -2), (-0.6, -1), (-0.5, 0), (-0.4, 0), (0.0, 0),
                                 (0.4, 0), (0.5, 1), (0.6, 1), (3.9, 4)];
    for (world, expected) in cases {
        assert_eq!(Node::from_world(*world, *world).x, *expected,
                   "мировая координата {world} должна попадать в клетку {expected}");
        assert_eq!(Node::from_world(*world, *world).y, *expected);
    }

    // Клетка -> мир -> клетка обязана быть стабильной, иначе NPC, стоящий
    // на границе клетки, будет «дёргаться» между двумя соседними клетками
    // каждый кадр.
    for x in -12..=12 {
        for y in -12..=12 {
            let node = Node::new(x, y);
            let (wx, wy) = node.to_world();
            assert_eq!(Node::from_world(wx, wy), node,
                       "округление туда-обратно сломало клетку {node:?}");
        }
    }
}

// ========================================================================
//  core::util — чистые математические помощники
// ========================================================================

#[test]
fn ndc_to_world_maps_window_centre_onto_the_camera() {
    use crate::core::util::ndc_to_world;

    let window = (1600.0, 800.0);
    let (cam_x, cam_y) = (3.0, -2.0);

    // Центр окна — это ровно позиция камеры, независимо от зума (map_size).
    // Если это разъедется, мышь и камера разъедутся по экрану.
    for map_size in [0.4, 0.8, 1.0, 2.5] {
        let (wx, wy) = ndc_to_world(window.0 / 2.0, window.1 / 2.0, window, map_size, cam_x, cam_y);
        assert!((wx - cam_x).abs() < 1e-4 && (wy - cam_y).abs() < 1e-4,
                "при map_size={map_size} центр окна дал ({wx},{wy}), а камера в ({cam_x},{cam_y})");
    }

    // Ось Y в мировых координатах смотрит вверх, а в пикселях окна — вниз,
    // поэтому верх окна (my = 0) должен быть ВЫШЕ центра мира.
    let (_, wy_top) = ndc_to_world(window.0 / 2.0, 0.0, window, 1.0, cam_x, cam_y);
    assert!(wy_top > cam_y, "верх окна (y={wy_top}) должен быть выше центра ({cam_y})");
    let (_, wy_bottom) = ndc_to_world(window.0 / 2.0, window.1, window, 1.0, cam_x, cam_y);
    assert!(wy_bottom < cam_y, "низ окна (y={wy_bottom}) должен быть ниже центра ({cam_y})");

    // map_size — это ЗУМ, а не размер карты: он входит в множитель масштаба
    // напрямую, поэтому большее значение приближает камеру (меньше мира в
    // кадре). Диапазон в игре — [ZOOM_MIN, ZOOM_MAX].
    use crate::core::constants::{ZOOM_MAX, ZOOM_MIN};
    let (_, wy_zoomed_in) = ndc_to_world(window.0 / 2.0, 0.0, window, ZOOM_MAX, cam_x, cam_y);
    let (_, wy_zoomed_out) = ndc_to_world(window.0 / 2.0, 0.0, window, ZOOM_MIN, cam_x, cam_y);
    assert!(wy_zoomed_out > wy_zoomed_in,
            "при отдалении ({wy_zoomed_out}) кадр должен покрывать больше мира, чем при приближении ({wy_zoomed_in})");
}

#[test]
fn ease_out_back_is_clamped_and_overshoots_once() {
    use crate::core::util::ease_out_back;

    // Границы перелёта: ease_out_back — это «0 в начале, 1 в конце с отскоком».
    assert!((ease_out_back(0.0) - 0.0).abs() < 1e-6, "в начале перелёта должно быть 0");
    assert!((ease_out_back(1.0) - 1.0).abs() < 1e-6, "в конце перелёта должно быть 1");

    // Значения вне [0,1] клампятся — иначе «поп» инвентаря улетел бы за экран.
    assert_eq!(ease_out_back(-5.0), ease_out_back(0.0));
    assert_eq!(ease_out_back(5.0), ease_out_back(1.0));
    assert_eq!(ease_out_back(0.5), ease_out_back(0.5));

    // Суть «back»: значение перелетает через 1 и возвращается. Иконки
    // инвентаря подпрыгивают именно этим эффектом, поэтому пик выше 1 —
    // признак того, что функция вообще работает как задумано.
    let peak = (0..=1000)
        .map(|i| ease_out_back(i as f32 / 1000.0))
        .fold(f32::MIN, f32::max);
    assert!(peak > 1.05, "перелёт без отскока (пик {peak}) — ease_out_back считается неверно");
    assert!(peak < 1.3, "отскок слишком большой ({peak}), иконки будут вылетать за клетки");
}

#[test]
fn inventory_index_flips_rows_so_grid_fills_bottom_up() {
    use crate::core::util::inventory_index;
    use crate::core::constants::{INVENTORY_COLS, INVENTORY_ROWS};

    // Сетка нумеруется сверху вниз, а список предметов снизу вверх, поэтому
    // первая строка отдаёт самые высокие индексы. Если flip пропадёт, вся
    // разметка инвентара (иконки, рамки, тултипы) съедет на две строки.
    assert_eq!(inventory_index(0, 0), (INVENTORY_ROWS - 1) * INVENTORY_COLS);
    assert_eq!(inventory_index(INVENTORY_ROWS - 1, 0), 0);

    // Диапазон индексов — ровно размер сетки, без дыр и пересечений.
    let mut all: Vec<i32> = (0..INVENTORY_ROWS)
        .flat_map(|row| (0..INVENTORY_COLS).map(move |col| inventory_index(row, col)))
        .collect();
    assert_eq!(all.len() as i32, INVENTORY_ROWS * INVENTORY_COLS);
    all.sort_unstable();
    assert_eq!(all, (0..INVENTORY_ROWS * INVENTORY_COLS).collect::<Vec<_>>(),
               "индексы сетки должны быть перестановкой 0..N без повторов");
}

#[test]
fn ui_fit_scale_never_leaves_its_clamp() {
    use crate::core::util::ui_fit_scale;

    // Реальные значения из сцен: игровой HUD шире меню.
    for &(aspect, half_width_world) in &[(0.5f32, 6.5f32), (1.0, 6.5), (16.0 / 9.0, 6.5),
                                        (1.0, 3.6), (4.0, 3.6), (0.25, 3.6), (21.0, 9.0)] {
        let s = ui_fit_scale(aspect, half_width_world);
        assert!((0.30..=1.0).contains(&s),
                "ui_fit_scale({aspect}, {half_width_world}) = {s} вышло за [0.30, 1.0]");
        assert!(s.is_finite(), "ui_fit_scale вернул {s}");
    }

    // На широком экране интерфейс не растягивается (потолок 1.0), на узком —
    // ужимается. Монотонность по aspect обязана сохраняться до потолка.
    assert_eq!(ui_fit_scale(4.0, 6.5), 1.0, "на широком экране нужен номинальный масштаб");
    assert!(ui_fit_scale(0.5, 6.5) < ui_fit_scale(1.0, 6.5),
            "узкий экран должен ужимать интерфейс сильнее широкого");
}

// ========================================================================
//  Виртуальные пути «текстура@WxH» (растянутые на несколько клеток)
// ========================================================================

#[test]
fn sized_path_parsing_splits_base_and_footprint() {
    use crate::ecs::Sprite;

    // Обычный атласный путь: размера в мире нет, берётся из scale.
    assert_eq!(Sprite::split_sized_path("assets/tex/decor/regular/box.png"),
               ("assets/tex/decor/regular/box.png", None));

    // Виртуальный путь много��леточного объекта (например big_lamp 1x2):
    // база + размер в клетках.
    assert_eq!(Sprite::split_sized_path("assets/tex/decor/light/big_lamp.png@1x2"),
               ("assets/tex/decor/light/big_lamp.png", Some((1.0, 2.0))));

    // Разделитель берётся ПОСЛЕДНИМ: в базовом пути уже может быть «@».
    assert_eq!(Sprite::split_sized_path("a/b@old@2x1"), ("a/b@old", Some((2.0, 1.0))),
               "нужен последний @, а не первый");

    // Мусор и нулевые/отрицательные размеры должны откатываться к атласу,
    // иначе Sprite::new построит quad с нулевым или вывернутым размером.
    for bad in ["a/b.png@", "a/b.png@2", "a/b.png@2x", "a/b.png@xx", "a/b.png@2xy",
                "a/b.png@0x1", "a/b.png@1x0", "a/b.png@-2x1", "a/b.png@0.0x0.0"] {
        let (base, size) = Sprite::split_sized_path(bad);
        assert_eq!(size, None, "путь {bad:?} не должен давать размер, а дал {size:?}");
        assert_eq!(base, "a/b.png", "при неудачном разборе база должна остаться прежней");
    }

    // Путь без базы — вроде валидный, но рисуется в никуда; парсер не должен
    // на этом паниковать.
    assert_eq!(Sprite::split_sized_path("@2x2"), ("", Some((2.0, 2.0))));
}

// ========================================================================
//  Токены карты -> текстура и кадр атласа
// ========================================================================

#[test]
fn every_map_token_resolves_to_an_in_bounds_atlas_frame() {
    use crate::data::map::token_to_texture;
    use crate::ecs::components::Season;

    // Полный список токенов из match в token_to_texture. Держим его руками
    // рядом с функцией: если добавить новый токен и забыть про этот тест,
    // он упадёт не сразу, а в игре отрисуется чёрным квадратом.
    const TOKENS: &str = ".@*mf~lHK123456PQZX,_AD=-hd^&WS/|(){}][:;o%Eqp";

    // Дополнительно прогоняем токены, реально встречающиеся в карте.
    let mut from_map: Vec<String> = include_str!("../../assets/map.txt")
        .lines()
        .flat_map(|line| line.chars().map(|c| c.to_string()))
        .collect();
    from_map.sort();
    from_map.dedup();

    for token in TOKENS.chars().map(|c| c.to_string()).chain(from_map) {
        for season in Season::all() {
            let (path, frame, count) = token_to_texture(&token, *season);
            assert!(path.ends_with(".png"), "токен {token:?} дал не текстуру: {path}");
            assert!(frame[0] >= 0 && frame[1] >= 0,
                    "токен {token:?} ({season:?}) дал отрицательный кадр {frame:?}");
            assert!(frame[0] < count[0] && frame[1] < count[1],
                    "токен {token:?} ({season:?}) дал кадр {frame:?} за пределами атласа {count:?}");
            assert!(count[0] > 0 && count[1] > 0,
                    "токен {token:?} ({season:?}) дал пустой атлас {count:?}");
        }
    }

    // Сезон меняет только текстуру травы, но не пол и не стены.
    let (_, winter_floor, _) = token_to_texture("0", Season::Winter);
    assert_eq!(winter_floor, [1, 1], "пол не должен зависеть от сезона");
    let (summer_grass, _, _) = token_to_texture(".", Season::Summer);
    let (winter_grass, _, _) = token_to_texture(".", Season::Winter);
    assert_ne!(summer_grass, winter_grass, "трава обязана быть сезонной");
}

// ========================================================================
//  Хит-тест прямоугольников UI
// ========================================================================

#[test]
fn is_inside_uses_exclusive_bounds() {
    use crate::ui::system::is_inside;

    // Центр и почти-центр внутри.
    assert!(is_inside(0.0, 0.0, 0.0, 0.0, 1.0, 1.0));
    assert!(is_inside(0.99, -0.99, 0.0, 0.0, 1.0, 1.0), "угол почти вплотную — ещё внутри");

    // Граница НЕ внутри: соседние кнопки, поставленные вплотную, иначе
    // перехватывали бы клики друг друга ровно на шов.
    assert!(!is_inside(1.0, 0.0, 0.0, 0.0, 1.0, 1.0), "верхняя граница должна быть снаружи");
    assert!(!is_inside(0.0, -1.0, 0.0, 0.0, 1.0, 1.0), "нижняя граница должна быть снаружи");
    assert!(!is_inside(1.5, 0.0, 0.0, 0.0, 1.0, 1.0));

    // Прямоугольник несимметричен: каждая ось проверяется своей половиной.
    assert!(is_inside(1.5, 0.0, 0.0, 0.0, 2.0, 0.5));
    assert!(!is_inside(0.0, 0.6, 0.0, 0.0, 2.0, 0.5));
}

// ========================================================================
//  Категории предметов и их списки
// ========================================================================

#[test]
fn object_categories_partition_the_inventory_tabs() {
    use crate::core::constants::*;
    use crate::data::placement::*;

    // Предикаты обязаны совпадать со своими списками — иначе предмет из
    // вкладки получит правила другой категории (ковёр поставят на стену).
    for &name in CARPET_NAMES.iter() {
        assert!(is_carpet_name(name), "{name} должен быть ковром");
    }
    for &name in INV_LIGHT.iter() {
        assert!(is_light_name(name), "{name} должен быть светом");
    }
    for &name in INV_WALLDECOR.iter() {
        assert!(is_wall_decor_name(name), "{name} должен быть настенным декором");
    }
    for &name in OUTDOOR_NAMES.iter() {
        assert!(is_outdoor_name(name), "{name} должен быть уличным декором");
    }
    for &name in FLOWER_NAMES.iter() {
        assert!(is_flower_name(name), "{name} должен быть цветком");
    }

    // Цветы — частный случай уличного декора: их можно ставить на траву.
    for &name in FLOWER_NAMES.iter() {
        assert!(is_outdoor_name(name), "{name} обязан быть и уличным декором");
    }

    // Таблицы не должны пересекаться: один предмет не может попасть на две
    // вкладки (иконка нарисуется дважды, а can_place_at получит две разные
    // категории от одного имени).
    let lists: [(&str, &[&str]); 4] = [
        ("регулярный", INV_REGULAR),
        ("ковры", INV_CARPETS),
        ("настенный декор", INV_WALLDECOR),
        ("свет", INV_LIGHT),
    ];
    for i in 0..lists.len() {
        for j in i + 1..lists.len() {
            for &a in lists[i].1 {
                assert!(!lists[j].1.contains(&a),
                        "{} ({a}) попал сразу в две вкладки: {} и {}",
                        a, lists[i].0, lists[j].0);
            }
        }
    }
    for &name in INV_CARPETS {
        assert!(!OUTDOOR_NAMES.contains(&name), "ковёр {name} не должен быть уличным декором");
    }
    for &name in INV_LIGHT {
        assert!(!OUTDOOR_NAMES.contains(&name), "свет {name} не должен быть уличным декором");
    }

    // Предметы стартового хотбара обязаны существовать в каталоге.
    // Требовать покупки для них нельзя только у части: table намеренно НЕ
    // входит в SHOP_EXEMPT — он в хотбаре как подсказка, но покупается.
    for (i, &name) in INITIAL_SLOTS_TESTS.iter().enumerate() {
        let slot = crate::data::make_slot(name);
        assert_eq!(slot.obj.name, name, "предмет хотбара №{i} ({name}) не найден в ALL_OBJECTS");
        assert!(slot.obj.price > 0, "у предмета {name} должна быть ненулевая цена");
    }

    // А вот базовые предметы магазина (коробки, стеллажи, касса, вывеска) —
    // единственные, что обязаны быть доступны бесплатно, иначе игрок не
    // сможет запустить магазин в принципе.
    for &name in crate::core::constants::SHOP_EXEMPT {
        assert!(!requires_shop(name),
                "{name} — базовый предмет, он не должен требовать покупки");
    }
}

// ========================================================================
//  Каталог предметов и ресурсы предметов на диске
// ========================================================================

#[test]
fn every_catalog_object_has_a_resolvable_icon() {
    use crate::core::util::slot_icon_path;

    // Иконка — это первое, что игрок видит у предмета; битый путь даёт
    // пустую клетку без диагностики. Путь строит slot_icon_path по
    // категории, поэтому проверяем именно его вывод.
    for obj in crate::data::ALL_OBJECTS {
        let icon = slot_icon_path(obj.name);
        assert!(icon.ends_with(".png"), "иконка {name} собралась в не-PNG: {icon}", name = obj.name);
        assert!(crate::core::asset::load_bytes(&icon).is_ok(),
                "нет файла иконки предмета {}: {}", obj.name, icon);
    }

    // Плюс все предметы вкладок инвентаря — они рисуются в сетке напрямую,
    // мимо каталога объектов (часть имён есть только в списках вкладок).
    let mut tab_names: Vec<&str> = INVENTORY_TAB_NAMES_TESTS.to_vec();
    tab_names.extend_from_slice(crate::core::constants::INV_OUTDOOR);
    for name in tab_names {
        let icon = slot_icon_path(name);
        assert!(crate::core::asset::load_bytes(&icon).is_ok(), "нет иконки {name}: {icon}");
    }
}

#[test]
fn shop_catalogue_matches_the_purchasable_object_list() {
    use crate::core::constants::SHOP_EXEMPT;
    use crate::data::{object_price, shop_item_names};

    let names = shop_item_names();

    // Магазин — это каталог минус стартовые предметы. Соответствие строится
    // в двух местах (shop_item_names и GameScene::shop_items по одному и тому
    // же ALL_OBJECTS), поэтому порядок и состав обязаны совпадать.
    let expected: Vec<&str> = crate::data::ALL_OBJECTS.iter()
        .map(|o| o.name)
        .filter(|n| !SHOP_EXEMPT.contains(n))
        .collect();
    assert_eq!(names, expected, "состав магазина разошёлся с каталогом объектов");

    // Всё, что продаётся, обязано иметь цену и иконку, а стартовое —
    // не продаваться.
    for &name in &names {
        assert!(crate::core::util::slot_icon_path(name).ends_with(".png"));
        let cfg = crate::scripts::config::BalanceConfig::load();
        assert!(object_price(name, &cfg) > 0, "товар {name} продаётся с неположительной ценой");
    }
    for &name in SHOP_EXEMPT {
        assert!(!names.contains(&name), "{name} не должен продаваться в магазине");
        assert!(!crate::data::placement::requires_shop(name),
                "{name} должен быть доступен без магазина");
    }
}

#[test]
fn balance_config_defaults_are_internally_consistent() {
    use crate::scripts::config::BalanceConfig;

    let cfg = BalanceConfig::default();

    // Порядок фаз суток: день -> закат -> ночь -> рассвет -> снова день.
    // Нарушение этого порядка (например в config.lua) ломает DayNightCycle:
    // ветки перекрываются и затенение прыгает.
    assert!(cfg.day_secs > 0.0, "день не может быть нулевой длительности");
    assert!(cfg.day_secs <= cfg.night_start_secs,
            "ночь не может начаться раньше конца дня ({day} > {night_start})",
            day = cfg.day_secs, night_start = cfg.night_start_secs);
    assert!(cfg.night_start_secs <= cfg.night_secs,
            "рассвет не может начаться раньше конца ночи ({night_start} > {night})",
            night_start = cfg.night_start_secs, night = cfg.night_secs);
    // Рассвет обязан успеть закончиться до конца цикла, иначе factor()
    // вернёт отрицательное затенение (1 - (cycle - night)/fade < 0).
    assert!(cfg.night_secs + cfg.fade_secs <= cfg.day_cycle_secs,
            "рассвет ({night}+{fade}) не помещается в цикл {cycle} — затенение уйдёт в минус",
            night = cfg.night_secs, fade = cfg.fade_secs, cycle = cfg.day_cycle_secs);
    assert!(cfg.fade_secs > 0.0, "нулевая длительность перехода даёт деление на ноль");

    // Пороги текстур ящика: по порядку, иначе средний ящик никогда не рисуется.
    assert!(cfg.box_tex_threshold_1 <= cfg.box_tex_threshold_2,
            "пороги текстур ящика идут в обратном порядке: {} > {}",
            cfg.box_tex_threshold_1, cfg.box_tex_threshold_2);

    // Запасы конфет не могут превышать вместимость.
    assert!(cfg.candies_start_food <= cfg.max_food_candies,
            "стартовая еда {} больше максимума {}",
            cfg.candies_start_food, cfg.max_food_candies);

    assert!(cfg.max_shoppers >= 1, "хотя бы один покупатель должен ходить в магазин");
    assert!(cfg.npc_speed > 0.0, "нулевая скорость NPC замораживает покупателей");
    assert!(cfg.day_cycle_secs > 0.0, "нулевой цикл суток даёт деление на ноль в time_string");
    assert!(!cfg.font_path.is_empty(), "путь к шрифту не должен быть пустым");
}

// Локальные срезы данных для тестов выше: держать списки рядом с тестом,
// чтобы при изменении каталога падение было в читаемом месте.
const INITIAL_SLOTS_TESTS: &[&str] = &["box", "sign", "rack", "table", "cassa"];
const INVENTORY_TAB_NAMES_TESTS: &[&str] = &["box", "sign", "rack", "table", "cassa", "ice_cream",
                                            "arcade_machine", "candies", "fence", "basement",
                                            "blue_carpet", "red_carpet", "green_carpet",
                                            "white_carpet", "black_carpet", "iron_panel",
                                            "gold_panel", "diamond_panel",
                                            "welcome", "fnaf", "watch",
                                            "lamp", "big_lamp"];
