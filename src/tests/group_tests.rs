// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  group_tests — групповые (multicell) объекты: раскладка по клеткам,
//  атласные кадры, слои, поиск по позиции и удаление группы.
// ========================================================================

use specs::WorldExt;

use crate::core::constants::{Z_CARPET, Z_DECOR, Z_LIGHT};
use crate::ecs::components::{SpriteComponent, Transform};
use crate::tests::helpers::{group_count, group_entities, new_ecs};

fn transform_of(ecs: &crate::EcsAdapter, entity: specs::Entity) -> [f32; 3] {
    ecs.world.read_storage::<Transform>().get(entity).unwrap().position
}

fn sprite_of(ecs: &crate::EcsAdapter, entity: specs::Entity) -> (
    std::sync::Arc<str>,
    [i32; 2],
    [i32; 2],
    f32,
) {
    let sprites = ecs.world.read_storage::<SpriteComponent>();
    let sprite = sprites.get(entity).unwrap();
    (
        std::sync::Arc::clone(&sprite.texture_path),
        sprite.texture_frame,
        sprite.texture_count,
        sprite.scale,
    )
}

#[test]
fn multicell_object_creates_one_entity_per_cell_with_atlas_frames() {
    let mut ecs = new_ecs();
    let gid = ecs.add_group_object(
        1, 2, 2, 2,
        "table", "assets/tex/decor/regular/table.png",
        [0, 0], [2, 2],
        false, false, false, &[],
    );

    let entities = group_entities(&ecs, gid);
    assert_eq!(entities.len(), 4);
    let expected = [
        ((1.0, 2.0), [0, 0]),
        ((1.0, 3.0), [0, 1]),
        ((2.0, 2.0), [1, 0]),
        ((2.0, 3.0), [1, 1]),
    ];
    for (entity, (pos, frame)) in entities.iter().zip(expected) {
        let t = transform_of(&ecs, *entity);
        assert_eq!((t[0], t[1]), pos);
        let (path, tex_frame, count, _) = sprite_of(&ecs, *entity);
        assert_eq!(path.as_ref(), "assets/tex/decor/regular/table.png");
        assert_eq!(tex_frame, frame);
        assert_eq!(count, [2, 2]);
    }
}

#[test]
fn spanning_object_uses_a_single_stretched_entity() {
    let mut ecs = new_ecs();
    // big_lamp: 1x2, но текстура одна (texture_count [1,1]).
    let gid = ecs.add_group_object(
        0, 0, 1, 2,
        "big_lamp", "assets/tex/decor/light/big_lamp.png",
        [0, 0], [1, 1],
        false, false, false, &[],
    );

    let entities = group_entities(&ecs, gid);
    assert_eq!(entities.len(), 1, "растянутый объект должен быть одной сущностью");
    let t = transform_of(&ecs, entities[0]);
    assert_eq!((t[0], t[1]), (0.0, 0.5));
    let (path, frame, count, _) = sprite_of(&ecs, entities[0]);
    assert_eq!(path.as_ref(), "assets/tex/decor/light/big_lamp.png@1x2");
    assert_eq!(frame, [0, 0]);
    assert_eq!(count, [1, 1]);
}

#[test]
fn group_layer_depends_on_category() {
    let mut ecs = new_ecs();

    let carpet = ecs.add_group_object(
        0, 0, 1, 1, "blue_carpet", "assets/tex/decor/carpets/carpet.png",
        [0, 0], [4, 4], true, false, false, &[],
    );
    let light = ecs.add_group_object(
        1, 0, 1, 1, "lamp", "assets/tex/decor/light/lamp.png",
        [0, 0], [1, 1], false, true, false, &[],
    );
    let decor = ecs.add_group_object(
        2, 0, 1, 1, "box", "assets/tex/decor/regular/box/box_0.png",
        [0, 0], [1, 1], false, false, false, &[],
    );

    for (gid, z) in [(carpet, Z_CARPET), (light, Z_LIGHT), (decor, Z_DECOR)] {
        for entity in group_entities(&ecs, gid) {
            assert_eq!(transform_of(&ecs, entity)[2], z);
        }
    }
}

#[test]
fn delete_group_removes_entities_components_and_metadata() {
    let mut ecs = new_ecs();
    let gid = ecs.add_group_object(
        0, 0, 2, 2, "table", "assets/tex/decor/regular/table.png",
        [0, 0], [2, 2], false, false, false, &[],
    );
    let entities = group_entities(&ecs, gid);
    assert_eq!(group_count(&ecs), 1);

    ecs.delete_group(gid);

    assert_eq!(group_count(&ecs), 0);
    assert!(ecs.find_group_at_position(0, 0).is_none());
    for entity in entities {
        assert!(ecs.world.read_storage::<Transform>().get(entity).is_none());
        assert!(ecs.world.read_storage::<SpriteComponent>().get(entity).is_none());
        assert!(ecs.world.read_storage::<crate::GroupComponent>().get(entity).is_none());
    }
    // Повторное удаление безопасно.
    ecs.delete_group(gid);
}

#[test]
fn find_group_at_position_uses_decor_then_light_then_carpet() {
    let mut ecs = new_ecs();
    let carpet = ecs.add_group_object(
        0, 0, 1, 1, "blue_carpet", "carpet", [0, 0], [4, 4], true, false, false, &[],
    );
    let light = ecs.add_group_object(
        0, 0, 1, 1, "lamp", "lamp", [0, 0], [1, 1], false, true, false, &[],
    );
    let decor = ecs.add_group_object(
        0, 0, 1, 1, "box", "box", [0, 0], [1, 1], false, false, false, &[],
    );

    assert_eq!(ecs.find_group_at_position(0, 0), Some(decor));
    ecs.delete_group(decor);
    assert_eq!(ecs.find_group_at_position(0, 0), Some(light));
    ecs.delete_group(light);
    assert_eq!(ecs.find_group_at_position(0, 0), Some(carpet));
    ecs.delete_group(carpet);
    assert_eq!(ecs.find_group_at_position(0, 0), None);
}

#[test]
fn find_group_at_position_checks_the_whole_rectangle() {
    let mut ecs = new_ecs();
    let gid = ecs.add_group_object(
        2, 3, 2, 2, "table", "table", [0, 0], [2, 2], false, false, false, &[],
    );

    for (x, y) in [(2, 3), (3, 3), (2, 4), (3, 4)] {
        assert_eq!(ecs.find_group_at_position(x, y), Some(gid), "клетка ({x},{y})");
    }
    for (x, y) in [(1, 3), (4, 3), (2, 2), (2, 5)] {
        assert_eq!(ecs.find_group_at_position(x, y), None, "клетка ({x},{y})");
    }
}
