// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  anim_tests — PanelAnim: открытие, закрытие, отмена и текстовые спрайты.
// ========================================================================

use specs::WorldExt;

use crate::ecs::components::SpriteComponent;
use crate::ecs::factory::create_sprite;
use crate::tests::helpers::new_ecs;
use crate::ui::anim::{AnimEvent, AnimPhase, PanelAnim, PANEL_CLOSE_DURATION, PANEL_OPEN_DURATION};
use crate::EcsAdapter;

fn sprite(ecs: &mut EcsAdapter, path: &str) -> specs::Entity {
    create_sprite(
        &mut ecs.world,
        0.0, 0.0, 0.0,
        path, [0, 0], [1, 1], 1.0, 1.0,
    )
}

fn scale(ecs: &EcsAdapter, entity: specs::Entity) -> f32 {
    ecs.world.read_storage::<SpriteComponent>().get(entity).unwrap().scale
}

fn alpha(ecs: &EcsAdapter, entity: specs::Entity) -> f32 {
    ecs.world.read_storage::<SpriteComponent>().get(entity).unwrap().alpha
}

#[test]
fn opening_animation_finishes_with_opened_event() {
    let mut ecs = new_ecs();
    let entity = sprite(&mut ecs, "assets/tex/ui/shop_panel.png");
    let mut anim = PanelAnim::new();

    assert!(!anim.is_active());
    anim.start_open(&mut ecs, vec![entity]);
    assert_eq!(anim.phase(), AnimPhase::Opening);
    assert!(anim.is_active());
    assert!((scale(&ecs, entity) - 0.6).abs() < 1e-6, "старт с масштаба 0.6");

    assert_eq!(anim.tick(&mut ecs, PANEL_OPEN_DURATION * 0.5), AnimEvent::None);
    let mid = scale(&ecs, entity);
    assert!(mid > 0.6 && mid <= 1.2, "масштаб в середине: {mid}");

    assert_eq!(anim.tick(&mut ecs, PANEL_OPEN_DURATION), AnimEvent::Opened);
    assert_eq!(anim.phase(), AnimPhase::Idle);
    assert!(!anim.is_active());
    assert!((scale(&ecs, entity) - 1.0).abs() < 1e-6);
}

#[test]
fn closing_animation_fades_and_deletes_entities() {
    let mut ecs = new_ecs();
    let entity = sprite(&mut ecs, "assets/tex/ui/shop_panel.png");
    let mut anim = PanelAnim::new();
    anim.start_close(&mut ecs, vec![entity]);
    assert_eq!(anim.phase(), AnimPhase::Closing);
    assert!(anim.is_closing());

    assert_eq!(anim.tick(&mut ecs, PANEL_CLOSE_DURATION * 0.5), AnimEvent::None);
    let a = alpha(&ecs, entity);
    assert!(a > 0.0 && a < 1.0, "альфа в середине закрытия: {a}");
    assert!(scale(&ecs, entity) < 1.0);

    assert_eq!(anim.tick(&mut ecs, PANEL_CLOSE_DURATION), AnimEvent::Closed);
    assert!(!anim.is_active());
    assert!(
        ecs.world.read_storage::<SpriteComponent>().get(entity).is_none(),
        "сущность должна быть удалена после закрытия"
    );
}

#[test]
fn text_sprites_fade_instead_of_scaling() {
    let mut ecs = new_ecs();
    let entity = sprite(&mut ecs, "__text__Settings");
    let mut anim = PanelAnim::new();

    anim.start_open(&mut ecs, vec![entity]);
    assert_eq!(alpha(&ecs, entity), 0.0);
    assert!((scale(&ecs, entity) - 1.0).abs() < 1e-6, "текст не масштабируется");

    assert_eq!(anim.tick(&mut ecs, PANEL_OPEN_DURATION * 0.5), AnimEvent::None);
    assert!((alpha(&ecs, entity) - 0.5).abs() < 1e-5);
    assert!((scale(&ecs, entity) - 1.0).abs() < 1e-6);

    assert_eq!(anim.tick(&mut ecs, PANEL_OPEN_DURATION), AnimEvent::Opened);
    assert!((alpha(&ecs, entity) - 1.0).abs() < 1e-5);
    assert!((scale(&ecs, entity) - 1.0).abs() < 1e-6);
}

#[test]
fn cancel_deletes_only_while_closing() {
    // Отмена закрытия удаляет сущности.
    let mut ecs = new_ecs();
    let entity = sprite(&mut ecs, "assets/tex/ui/shop_panel.png");
    let mut anim = PanelAnim::new();
    anim.start_close(&mut ecs, vec![entity]);
    anim.cancel(&mut ecs);
    assert!(!anim.is_active());
    assert!(ecs.world.read_storage::<SpriteComponent>().get(entity).is_none());

    // Отмена открытия сущности не трогает.
    let mut ecs = new_ecs();
    let entity = sprite(&mut ecs, "assets/tex/ui/shop_panel.png");
    let mut anim = PanelAnim::new();
    anim.start_open(&mut ecs, vec![entity]);
    anim.cancel(&mut ecs);
    assert!(!anim.is_active());
    assert!(ecs.world.read_storage::<SpriteComponent>().get(entity).is_some());
}

#[test]
fn idle_tick_is_a_noop() {
    let mut ecs = new_ecs();
    let mut anim = PanelAnim::new();
    assert_eq!(anim.tick(&mut ecs, 10.0), AnimEvent::None);
    assert_eq!(anim.phase(), AnimPhase::Idle);
    assert!(!anim.is_active());
}
