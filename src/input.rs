use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

use crate::EcsAdapter;
use crate::Slot;
use specs::*;

use crate::{add, remove};

pub fn do_input(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &WinitInputHelper,
    ecs: &mut EcsAdapter,
    slots: &mut Vec<Slot>,
    act_slot: i32,
    mode: i32,
    map_size: f32,

    cursor_entity: Entity,
    icon_button: Entity,
) -> (i32, i32, f32) {

    let mut new_size = map_size;
    let mut new_mode = mode;
    let mut new_act_slot = act_slot;

    let scroll = input.scroll_diff();
    if scroll != (0.0, 0.0) {
        if scroll.1 > 0.0 && map_size < 0.8{
            new_size += 0.2;
        } else if scroll.1 < 0.0 && map_size > 0.5{
            new_size -= 0.2;
        }
    }

    if input.key_pressed(KeyCode::KeyF) {
        match mode {
            0 => {},
            1 => {
                add(ecs, slots, act_slot, cursor_entity);
            },
            2 => {
                remove(ecs, cursor_entity);
            },
            _ => {}
        }
    }

    if input.key_pressed(KeyCode::Tab) {
        if new_mode == 2{
            new_mode = 0
        }else{
            new_mode += 1;
        }

        match new_mode{
            0 => ecs.update_sprite_texture(cursor_entity, "tex/cursor/def_cursor.png"),
            1 => ecs.update_sprite_texture(cursor_entity, "tex/cursor/cursor.png"),
            2 => ecs.update_sprite_texture(cursor_entity, "tex/cursor/del_cursor.png"),
            _ => ()
        }

        match new_mode{
            0 => ecs.update_sprite_texture(icon_button, "tex/ui/mode/standart_mode.png"),
            1 => ecs.update_sprite_texture(icon_button, "tex/ui/mode/build_mode.png"),
            2 => ecs.update_sprite_texture(icon_button, "tex/ui/mode/del_mode.png"),
            _ => ()
        }
    }

    if input.key_pressed(KeyCode::KeyQ) {
        if new_act_slot >= 0 && (new_act_slot as usize) < slots.len() {
            slots[new_act_slot as usize].active = false;
        }

        if new_act_slot == 4 {
            new_act_slot = 0;
            //game.ui.cursor_slot.translation[0] = -4.0;
        } else {
            new_act_slot += 1;
            //game.ui.cursor_slot.translation[0] += 1.0;
        }

        // game.ui.cursor_slot.build_buffers(&queue); 
        
        // if new_act_slot >= 0 && (new_act_slot as usize) < slots.len() {
        //     slots[new_act_slot as usize].active = true;
        // }
    }

    let (x, y) = ecs.get_transform_position(cursor_entity);
    let speed = 1.0;

    if input.key_pressed(KeyCode::KeyW) {
        if y < 4.0{
            ecs.update_transform_position(cursor_entity, x, y + speed);
        }
    }

    if input.key_pressed(KeyCode::KeyS) {
        if y > -4.0{
            ecs.update_transform_position(cursor_entity, x, y - speed);
        }
    }

    if input.key_pressed(KeyCode::KeyA) {
        if x > -4.0{
            ecs.update_transform_position(cursor_entity, x - speed, y);
        }
    }

    if input.key_pressed(KeyCode::KeyD) {
        if x < 4.0{
            ecs.update_transform_position(cursor_entity, x + speed, y); 
        }
    }

    (new_act_slot, new_mode, new_size)
}