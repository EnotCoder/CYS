use specs::{Entity, WorldExt};
use crate::{EcsAdapter, Slot};
use crate::slot_object::{add, remove};
use crate::constants::*;
use crate::ecs::components::{FoodStorage, ObjectTag, TotalFood};

pub fn try_interact(ecs: &mut EcsAdapter, cursor: Entity) -> bool {
    let (cx, cy) = ecs.get_transform_position(cursor);
    let gx = cx as i32;
    let gy = cy as i32;
    if let Some(gid) = ecs.find_group_at_position(gx, gy) {
        let first_entity = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&gid).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = first_entity {
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
                if name == "box" {
                    if let Some((count, _)) = food_storage {
                        if count > 0 {
                            ecs.world.write_storage::<FoodStorage>().get_mut(entity).map(|s| {
                                s.food_count = 0;
                            });
                            ecs.world.write_resource::<TotalFood>().0 += count;
                            ecs.update_object_textures();
                        }
                    }
                } else if name == "rack" {
                    if let Some((count, max)) = food_storage {
                        if count < max {
                            let take = {
                                let mut total = ecs.world.write_resource::<TotalFood>();
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
                            }
                        }
                    }
                }
            }

            let sprites = ecs.world.read_storage::<crate::SpriteComponent>();
            if let Some(sprite) = sprites.get(entity) {
                if sprite.texture_path.contains("arcade_machine") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn cycle_mode(mode: i32, ecs: &mut EcsAdapter, cursor: Entity, icon: Entity) -> i32 {
    let new_mode = if mode == 2 { 0 } else { mode + 1 };
    ecs.update_sprite_texture(cursor, CURSOR_TEX[new_mode as usize]);
    ecs.update_sprite_texture(icon, MODE_ICON_TEX[new_mode as usize]);
    new_mode
}

pub fn do_interact(
    ecs: &mut EcsAdapter,
    cursor_entity: Entity,
    mode: i32,
    slots: &mut Vec<Slot>,
    act_slot: i32,
) -> bool {
    let mut show_ilm = false;
    match mode {
        0 => { show_ilm = try_interact(ecs, cursor_entity); }
        1 => add(ecs, slots, act_slot, cursor_entity),
        2 => { remove(ecs, cursor_entity); }
        _ => {}
    }
    show_ilm
}
