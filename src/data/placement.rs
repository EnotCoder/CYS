use specs::{Entity, WorldExt};
use crate::EcsAdapter;
use super::Slot;

pub fn is_carpet_name(name: &str) -> bool {
    crate::constants::CARPET_NAMES.contains(&name)
}

pub fn is_wall_decor_name(name: &str) -> bool {
    crate::constants::INV_WALLDECOR.contains(&name)
}

pub fn is_outdoor_name(name: &str) -> bool {
    crate::constants::OUTDOOR_NAMES.contains(&name)
}

pub fn is_flower_name(name: &str) -> bool {
    crate::constants::FLOWER_NAMES.contains(&name)
}

pub fn add(ecs: &mut EcsAdapter, slots: &mut Vec<Slot>, act_slot: i32, cursor_entity: Entity) {
    let active_slot = &slots[act_slot as usize].obj;
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    let is_carpet = is_carpet_name(active_slot.name);
    let is_wall_decor = is_wall_decor_name(active_slot.name);
    let is_outdoor = is_outdoor_name(active_slot.name);
    let is_flower = is_flower_name(active_slot.name);

    if ecs.can_place_at(
        cursor_x as i32, cursor_y as i32,
        active_slot.width, active_slot.height,
        is_carpet, is_wall_decor, is_outdoor, is_flower,
    ) {
        ecs.clear_cursor_preview();
        let group_id = ecs.add_group_object(
            cursor_x as i32, cursor_y as i32,
            active_slot.width, active_slot.height,
            active_slot.path,
            active_slot.texture_frame,
            active_slot.texture_count,
            is_carpet,
            active_slot.animated,
            active_slot.frame_paths,
        );
        let first = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = first {
            use specs::WorldExt;
            ecs.world.write_storage::<crate::ObjectTag>().insert(entity, crate::ObjectTag {
                name: active_slot.name.to_string(),
            }).ok();
            if active_slot.name == "box" {
                ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                    food_count: 0,
                    max_food: 20,
                }).ok();
            } else if active_slot.name == "rack" {
                ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                    food_count: 0,
                    max_food: 15,
                }).ok();
            } else if active_slot.name == "fence" || active_slot.name == "street_fence" {
                ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent { name: active_slot.name.to_string() }).ok();
            }
        }
    }
}

pub fn remove(ecs: &mut EcsAdapter, cursor_entity: Entity) -> bool {
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    if let Some(group_id) = ecs.find_group_at_position(cursor_x as i32, cursor_y as i32) {
        ecs.delete_group(group_id);
        true
    } else {
        false
    }
}
