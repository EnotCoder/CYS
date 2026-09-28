// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Дверь магазина: анимация открытия/закрытия
// ========================================================================
//  Дверь — это четыре тайла карты с токеном "E" (см. data/map/mod.rs),
//  а не объект инвентаря. Прогресс анимации живёт в ресурсе мира
//  DoorProgress, а на спрайты тайлов его отражает update_door_textures.
//
//  Цикл: Closed -> Opening (шаги 1..3) -> Open -> Closing (3..0) -> Closed.
//  Дверь открывается, когда рядом с проёмом стоит покупатель (ShopperTag),
//  и закрывается, когда он ушёл. Пока дверь открывается, она всегда
//  проходима для поиска пути — load_walkable_cells не зависит от анимации,
//  иначе уже построенные маршруты стали бы невалидными.

use specs::{WorldExt, Join};
use crate::EcsAdapter;
use crate::core::constants::*;
use crate::ecs::components::{DoorProgress, DoorState, ShopperTag, Transform};

/// Число шагов распахивания: 0 — закрыта, 3 — полностью открыта.
const MAX_STEP: i32 = 3;

/// Ежекадровая логика двери: решает, открываться ли, и насколько.
/// Вызывается сценой, пока не пауза.
pub fn tick_door(ecs: &mut EcsAdapter, dt: f64) {
    // На других уровнях (подвал) дверной проём не загружен — выходим сразу.
    if !ecs.map_entities.contains_key(&(DOOR_X_MIN, DOOR_Y_MIN)) {
        return;
    }

    let want_open = shopper_near_door(ecs);
    let (state, step, mut timer) = {
        let p = ecs.world.read_resource::<DoorProgress>();
        (p.state, p.step, p.timer)
    };

    let (new_state, new_step, new_timer) = match state {
        DoorState::Closed if want_open => (DoorState::Opening, 0, 0.0),
        DoorState::Opening => {
            timer += dt;
            if timer >= DOOR_STEP_OPEN_SECS {
                if step >= MAX_STEP {
                    (DoorState::Open, MAX_STEP, 0.0)
                } else {
                    (DoorState::Opening, step + 1, 0.0)
                }
            } else {
                (DoorState::Opening, step, timer)
            }
        }
        DoorState::Open if !want_open => (DoorState::Closing, MAX_STEP, 0.0),
        DoorState::Closing => {
            timer += dt;
            if timer >= DOOR_STEP_CLOSE_SECS {
                if step <= 0 {
                    (DoorState::Closed, 0, 0.0)
                } else {
                    (DoorState::Closing, step - 1, 0.0)
                }
            } else {
                (DoorState::Closing, step, timer)
            }
        }
        DoorState::Open | DoorState::Closed => (state, step, timer),
    };

    if new_state != state || new_step != step {
        ecs.update_door_textures(new_step);
    }
    let mut p = ecs.world.write_resource::<DoorProgress>();
    p.state = new_state;
    p.step = new_step;
    p.timer = new_timer;
}

/// Есть ли покупатель рядом с дверным проёмом. Работает для всех NPC
/// с маркером ShopperTag, поэтому не важно, кто именно пришёл.
fn shopper_near_door(ecs: &EcsAdapter) -> bool {
    let center_x = (DOOR_X_MIN + DOOR_X_MAX) as f32 * 0.5;
    let center_y = (DOOR_Y_MIN + DOOR_Y_MAX) as f32 * 0.5;
    let r2 = DOOR_TRIGGER_RADIUS * DOOR_TRIGGER_RADIUS;
    let transforms = ecs.world.read_storage::<Transform>();
    let shoppers = ecs.world.read_storage::<ShopperTag>();
    (&transforms, &shoppers)
        .join()
        .any(|(t, _tag)| {
            let dx = t.position[0] - center_x;
            let dy = t.position[1] - center_y;
            dx * dx + dy * dy <= r2
        })
}
