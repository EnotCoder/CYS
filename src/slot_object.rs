use crate::EcsAdapter;
use specs::*;

pub struct Slot{
    pub obj: Object,
    pub active: bool,
}

pub struct Object{
    pub width: i32,
    pub height: i32,
    pub name: String,
    pub path: String,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
}

pub fn add(
    ecs: &mut EcsAdapter,
    slots: &mut Vec<Slot>,
    act_slot: i32,
    cursor_entity: Entity,
){
    let active_slot = &slots[act_slot as usize].obj;
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    let cursor_x = cursor_x as i32;
    let cursor_y = cursor_y as i32;
    let width = active_slot.width;
    let height = active_slot.height;
    let path = &active_slot.path;
    let texture_frame = active_slot.texture_frame;
    let texture_count = active_slot.texture_count;
    let is_carpet = active_slot.name == "carpet";
    
    // Единый вызов для всех объектов
    if ecs.can_place_at(cursor_x, cursor_y, width, height, is_carpet) {
        ecs.add_group_object(
            cursor_x as f32,
            cursor_y as f32,
            width,
            height,
            path,
            texture_frame,
            texture_count,
            is_carpet,
        );
    }
}

pub fn remove(
    ecs: &mut EcsAdapter,
    cursor_entity: Entity,
) -> bool {
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    let cursor_x = cursor_x as i32;
    let cursor_y = cursor_y as i32;
    
    if let Some(group_id) = ecs.find_group_at_position(cursor_x, cursor_y) {
        ecs.delete_group(group_id);
        true
    } else {
        false
    }
}


pub fn get_slot_vec() -> Vec<Slot>{
    vec![
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("box"), 
                path: String::from("tex/decor/box.png"),
                texture_frame: [0, 0], texture_count: [1, 1],
            },
            active: true,
        },
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("carpet"),
                path: String::from("tex/decor/carpet.png"),
                texture_frame: [0, 0], texture_count: [2, 2],
            },
            active: false,
        },
        Slot{
            obj: Object {
                width: 1, height: 1, name: String::from("sign"),
                path: String::from("tex/decor/sign.png"),
                texture_frame: [0, 0], texture_count: [1, 1],
            },
            active: false,
        },
        Slot{
            obj: Object {
                width: 1, height: 2, name: String::from("rack"),
                path: String::from("tex/decor/rack.png"),
                texture_frame: [0, 1], texture_count: [1, 2],
            },
            active: false,
        },
        Slot{
            obj: Object {
                width: 2, height: 1, name: String::from("table"),
                path: String::from("tex/decor/table.png"),
                texture_frame: [0, 0], texture_count: [2, 1],
            },
            active: false,
        },
    ]
}