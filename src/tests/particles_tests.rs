// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  particles_tests — симуляция партиклов и события установки/удаления.
//  Симуляция не требует GPU: проверяются только данные и ECS-очередь.
// ========================================================================

use crate::ecs::particles::{
    object_color, BurstKind, ParticleSystem, PlacementFxEvent, MAX_ACTIVE_PARTICLES,
};
use crate::tests::helpers::{ecs_with_map, new_ecs};

fn event(kind: BurstKind) -> PlacementFxEvent {
    PlacementFxEvent {
        kind,
        x: 2.0,
        y: 3.0,
        width: 2,
        height: 1,
        color: [12, 34, 56],
    }
}

#[test]
fn place_burst_uses_place_preset_and_stays_near_object() {
    let mut ecs = new_ecs();
    let mut fx = ParticleSystem::new();

    fx.burst(&mut ecs, event(BurstKind::Place));

    let snapshot = fx.snapshot();
    assert_eq!(snapshot.len(), 12, "установка должна давать 12 частиц");
    assert!(
        snapshot.iter().all(|p| p.kind == BurstKind::Place),
        "вспышка должна состоять только из частиц установки"
    );
    assert!(
        snapshot.iter().all(|p| p.color == [12, 34, 56]),
        "частицы должны наследовать цвет предмета"
    );
    assert!(
        snapshot
            .iter()
            .all(|p| (p.x - 2.0).abs() <= 0.8 && (p.y - 3.0).abs() <= 0.4),
        "частицы должны появляться в пределах объекта"
    );
}

#[test]
fn particle_texture_is_one_solid_pixel_square() {
    let rgba = ParticleSystem::debug_texture_rgba([10, 20, 30]);

    assert_eq!(rgba.len(), 4 * 4 * 4, "квадрат 4×4 RGBA");
    assert!(
        rgba.chunks_exact(4)
            .all(|pixel| pixel == &[10, 20, 30, 255]),
        "каждый пиксель должен быть сплошным цветом предмета"
    );
}

#[test]
fn tick_moves_particles_fades_them_and_removes_expired() {
    let mut ecs = new_ecs();
    let mut fx = ParticleSystem::new();
    fx.burst(&mut ecs, event(BurstKind::Place));
    let before = fx.snapshot();

    fx.tick(0.1);
    let after = fx.snapshot();

    assert!(
        after.iter().zip(before.iter()).all(|(a, b)| a.y > b.y),
        "частицы установки должны лететь вверх"
    );
    assert!(
        after.iter().zip(before.iter()).all(|(a, b)| a.alpha < b.alpha),
        "альфа должна уменьшаться со временем"
    );

    fx.tick(1.0);
    assert!(fx.is_empty(), " частицы должны исчезнуть после TTL");
}

#[test]
fn many_bursts_never_exceed_the_dynamic_slot_budget() {
    let mut ecs = new_ecs();
    let mut fx = ParticleSystem::new();

    for _ in 0..10 {
        fx.burst(&mut ecs, event(BurstKind::Remove));
    }

    assert_eq!(fx.len(), MAX_ACTIVE_PARTICLES, "превышен лимит частиц");
}

#[test]
fn placement_and_removal_push_one_event_each_with_group_center_and_item_color() {
    let mut ecs = ecs_with_map();
    let slot = crate::data::make_slot("rack");
    let (width, height) = (slot.obj.width, slot.obj.height);
    let expected_color = object_color(
        "rack",
        slot.obj.path,
        slot.obj.texture_frame,
        slot.obj.texture_count,
    );
    let mut slots = crate::data::get_slot_vec();
    slots[0] = slot;

    crate::data::add(&mut ecs, &mut slots, 0, 0, 3);

    assert_eq!(ecs.pending_placement_fx.len(), 1, "нет события установки");
    let placed = ecs.pending_placement_fx[0];
    assert_eq!(placed.kind, BurstKind::Place);
    assert_eq!(placed.x, 0.0 + (width - 1) as f32 * 0.5);
    assert_eq!(placed.y, 3.0 + (height - 1) as f32 * 0.5);
    assert_eq!(placed.color, expected_color, "цвет должен браться из предмета");

    assert!(crate::data::remove(&mut ecs, 0, 3), "объект не удалился");
    assert_eq!(ecs.pending_placement_fx.len(), 2, "нет события удаления");
    let removed = ecs.pending_placement_fx[1];
    assert_eq!(removed.kind, BurstKind::Remove);
    assert_eq!(removed.x, placed.x);
    assert_eq!(removed.y, placed.y);
    assert_eq!(removed.color, expected_color, "при сносе нужен цвет предмета");

    ecs.clear_world();
    assert!(
        ecs.pending_placement_fx.is_empty(),
        "очистка мира должна чистить очередь FX"
    );
}
