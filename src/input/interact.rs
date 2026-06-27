use specs::{Entity, WorldExt};
use crate::{EcsAdapter, Slot};
use crate::slot_object::{add, remove};
use crate::constants::*;
use crate::ecs::components::{BoxStorage, TotalFood};

pub fn try_interact(ecs: &mut EcsAdapter, cursor: Entity) -> bool {
    let (cx, cy) = ecs.get_transform_position(cursor);
    let gx = cx as i32;
    let gy = cy as i32;
    if let Some(gid) = ecs.find_group_at_position(gx, gy) {
        let mut collect_food = false;
        {
            let storage = ecs.world.read_resource::<BoxStorage>();
            if let Some(box_data) = storage.boxes.get(&gid) {
                if box_data.food_count > 0 {
                    collect_food = true;
                }
            }
        }
        if collect_food {
            let food = {
                let mut storage = ecs.world.write_resource::<BoxStorage>();
                if let Some(data) = storage.boxes.get_mut(&gid) {
                    let f = data.food_count;
                    data.food_count = 0;
                    f
                } else {
                    0
                }
            };
            ecs.world.write_resource::<TotalFood>().0 += food;
            ecs.update_box_textures();
        }

        let group = ecs.world.read_resource::<crate::GroupInfoResource>();
        if let Some(info) = group.groups.get(&gid) {
            if let Some(first) = info.entities.first() {
                if let Some(sprite) = ecs.world.read_storage::<crate::SpriteComponent>().get(*first) {
                    if sprite.texture_path.contains("arcade_machine") {
                        return true;
                    }
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
