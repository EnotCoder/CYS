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
    let is_floor = active_slot.name == "floor";

    let gx = cursor_x as i32;
    let gy = cursor_y as i32;

    if ecs.can_place_at(
        gx, gy,
        active_slot.width, active_slot.height,
        is_carpet, is_wall_decor, is_outdoor, is_flower, is_floor,
    ) {
        ecs.clear_cursor_preview();
        let group_id = ecs.add_group_object(
            gx, gy,
            active_slot.width, active_slot.height,
            active_slot.path,
            active_slot.texture_frame,
            active_slot.texture_count,
            is_carpet,
            active_slot.animated,
            active_slot.frame_paths,
            is_floor,
        );

        if is_floor && !ecs.map_grid.is_empty() {
            for i in 0..active_slot.width {
                for j in 0..active_slot.height {
                    let cx = gx + i;
                    let cy = gy + j;
                    let file_col = cx + 21;
                    let file_row = 14 - cy;
                    if file_row >= 0 && file_row < ecs.map_grid.len() as i32 &&
                       file_col >= 0 && file_col < ecs.map_grid[file_row as usize].len() as i32
                    {
                        let token = ecs.map_grid[file_row as usize][file_col as usize].clone();
                        if token != "0" {
                            ecs.map_grid[file_row as usize][file_col as usize] = "0".to_string();
                            ecs.floor_positions.insert((cx, cy));
                            ecs.outdoor_positions.remove(&(cx, cy));
                            ecs.flower_positions.remove(&(cx, cy));
                            if let Some(&map_entity) = ecs.map_entities.get(&(cx, cy)) {
                                ecs.update_sprite_texture(map_entity, "tex/map/floor.png");
                                let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
                                if let Some(sprite) = sprites.get_mut(map_entity) {
                                    sprite.texture_frame = [0, 0];
                                    sprite.texture_count = [2, 2];
                                }
                            }
                        }
                    }
                }
            }
            ecs.save_map_grid();
        }

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
