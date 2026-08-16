// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::{Entity, WorldExt};
use crate::{EcsAdapter, data::Slot};
use crate::data::{add, remove};
use crate::core::constants::*;
use crate::ecs::components::{FoodStorage, ObjectTag, TotalFood};

// Взаимодействие с объектом в клетке (gx, gy): коробка, стеллаж, подвал и аркады
// Возврат: 2 — запрос смены уровня, 1 — показать подсветку, 0 — ничего
pub fn try_interact(ecs: &mut EcsAdapter, gx: i32, gy: i32) -> i32 {
    if let Some(gid) = ecs.find_group_at_position(gx, gy) {
        // Берём первый объект группы — все объекты группы считаются одним «составным» объектом
        let first_entity = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&gid).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = first_entity {
            // Читаем имя объекта и данные о еде (для продажи/пополнения)
            let mut obj_name = None;
            let mut food_storage = None;
            {
                let tags = ecs.world.read_storage::<ObjectTag>();
                if let Some(tag) = tags.get(entity) {
                    obj_name = Some(tag.name.clone());
                }
                let foods = ecs.world.read_storage::<FoodStorage>();
                if let Some(storage) = foods.get(entity) {
                    food_storage = Some((storage.food_count, storage.max_food));
                }
            }
            if let Some(name) = obj_name {
                // Коробка: продать хранящуюся еду — вся накопленная еда уходит в TotalFood
                if name == "box" {
                    if let Some((count, _)) = food_storage {
                        if count > 0 {
                            ecs.world.write_storage::<FoodStorage>().get_mut(entity).map(|s| {
                                s.food_count = 0;
                            });
                            ecs.world.write_resource::<TotalFood>().0 += count;
                            ecs.update_object_textures();
                            crate::audio::play("sell");
                        }
                    }
                // Стеллаж: пополнение еды из общего запаса, пока не заполнится
                } else if name == "rack" {
                    if let Some((count, max)) = food_storage {
                        if count < max {
                            let take = {
                                let mut total = ecs.world.write_resource::<TotalFood>();
                                // Берём сколько не хватает, но не больше накопленного
                                let can_take = max - count;
                                let take = can_take.min(total.0);
                                total.0 -= take;
                                take
                            };
                            if take > 0 {
                                ecs.world.write_storage::<FoodStorage>().get_mut(entity).map(|s| {
                                    s.food_count += take;
                                });
                                ecs.update_object_textures();
                                crate::audio::play("coin");
                                ecs.pending_food_adds.push(gid);
                            }
                        }
                    }
                // Подвал: переход на следующий уровень (сигнал для сцены)
                } else if name == "basement" {
                    return 2; // signal: switch level
                }
            }

            // Аркадный автомат: возвращаем 1, чтобы подсветить сцену продажи
            let sprites = ecs.world.read_storage::<crate::SpriteComponent>();
            if let Some(sprite) = sprites.get(entity) {
                if sprite.texture_path.contains("arcade_machine") {
                    return 1;
                }
            }
        }
    }
    0
}

// Циклическое переключение режимов (0 -> 1 -> 2 -> 0) с обновлением иконок
pub fn cycle_mode(mode: i32, ecs: &mut EcsAdapter, cursor: Entity, icon: Entity) -> i32 {
    let new_mode = if mode == 2 { 0 } else { mode + 1 };
    // Меняем текстуру курсора и иконку режима в хотбаре
    ecs.update_sprite_texture(cursor, CURSOR_TEX[new_mode as usize]);
    ecs.update_sprite_texture(icon, MODE_ICON_TEX[new_mode as usize]);
    crate::audio::play("click");
    new_mode
}

// Точка входа клика в мире: действие зависит от текущего режима
pub fn do_interact(
    ecs: &mut EcsAdapter,
    gx: i32,
    gy: i32,
    mode: i32,
    slots: &mut Vec<Slot>,
    act_slot: i32,
) -> i32 {
    let mut result = 0;
    match mode {
        // Режим просмотра: обычное взаимодействие с объектами
        0 => { result = try_interact(ecs, gx, gy); }
        // Режим расстановки: разместить выбранный предмет
        1 => add(ecs, slots, act_slot, gx, gy),
        // Режим удаления: убрать объект с клетки
        2 => { remove(ecs, gx, gy); }
        _ => {}
    }
    result
}
