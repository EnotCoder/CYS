use crate::Sprite;
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
// pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, slots: Vec<Slot>) -> Self{
//     let mut mode_icon = 
//         Sprite::new(&device, &queue, "tex/ui/mode/standart_mode.png", [0,0], [1,1]);

//     mode_icon.translation = [4.0, -4.0, 0.0, 1.0];
//     mode_icon.build_buffers(&queue);  // ← изменено

//     let mut slots_icons: Vec<Sprite> = vec![];

    
//     let mut index = 0;
//     for slot in slots{
//         slots_icons.push(Sprite::new(&device, &queue, 
//             &format!("tex/ui/icon_slots/{}.png",slot.obj.name), 
//             [0,0], [1,1]));

//         slots_icons[index as usize].translation = [-4.0 + index as f32, -4.0, 0.0, 1.0];
//         slots_icons[index as usize].build_buffers(&queue);  // ← изменено

//         index += 1;
//     }

//     let mut cursor_slot = Sprite::new(&device, &queue, 
//         "tex/ui/icon_slots/cursor.png", [0,0], [1,1]);
//     cursor_slot.translation = [-4.0, -4.0, 0.0, 1.0];
//     cursor_slot.build_buffers(&queue);  // ← изменено

//     Self {
//         mode_icon,
//         slots_icons,
//         cursor_slot,
//     }
// }


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