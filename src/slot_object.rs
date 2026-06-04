use crate::Sprite;

pub struct Slot{
    pub id: i32,
    pub obj: Object,
    pub active: bool,
}

pub struct GroupInfo {
    pub blocks: Vec<usize>,
    pub width: i32,
    pub height: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub is_carpet: bool,
}

pub struct Object{
    pub sprite: Sprite,
    pub width: i32,
    pub height: i32,
    pub name: String,
}

//game
pub struct Ui{
    pub mode_icon: Sprite,
    pub slots_icons: Vec<Sprite>,
    pub cursor_slot: Sprite,
}

impl Ui{
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, slots: Vec<Slot>) -> Self{
        let mut mode_icon = 
            Sprite::new(&device, &queue, "tex/ui/mode/standart_mode.png", [0,0], [1,1]);

        mode_icon.translation = [4.0, 4.0, 0.0, 1.0];
        mode_icon.build_buffers(&device);

        let mut slots_icons: Vec<Sprite> = vec![];

        
        let mut index = 0;
        for slot in slots{
            slots_icons.push(slot.obj.sprite);

            slots_icons[index as usize].translation = [-4.0 + index as f32, -4.0, 0.0, 1.0];
            slots_icons[index as usize].build_buffers(&device);

            index += 1;
        }

        let mut cursor_slot = Sprite::new(&device, &queue, "tex/cursor/def_cursor.png", [0,0], [1,1]);
        cursor_slot.translation = [-4.0, -4.0, 0.0, 1.0];
        cursor_slot.build_buffers(&device);

        Self {
            mode_icon,
            slots_icons,
            cursor_slot,
        }
    }
}

pub struct GameObjects {
    pub cursor: Sprite,
    pub map: Vec<Sprite>,
    pub decor: Vec<Sprite>,
    pub carpets: Vec<Sprite>,
    pub groups: Vec<GroupInfo>,
    pub ui: Ui,
}

pub fn add(
    device: &wgpu::Device, 
    queue: &wgpu::Queue,
    game: &mut GameObjects,
    slots: &mut Vec<Slot>,
    act_slot: i32,
){
    let active_slot = &slots[act_slot as usize];
    let cursor_x = game.cursor.translation[0] as i32;
    let cursor_y = game.cursor.translation[1] as i32;
    let width = active_slot.obj.width;
    let height = active_slot.obj.height;
    
    // Проверяем можно ли поставить объект
    if can_place_object(&game, cursor_x, cursor_y, width, height, act_slot) {
        let mut block_indices = Vec::new();
        
        // Создаём блоки для каждой клетки
        for i in 0..width {
            for j in 0..height {
                let frame = &active_slot.obj.sprite.texture_frame;

                let mut block = Sprite::new(
                    &device, &queue,
                    &active_slot.obj.sprite.texture_path,
                    [width-1-i + frame[0], height-j-1 + frame[1]],
                    active_slot.obj.sprite.texture_count,
                );
                block.translation = [
                    (cursor_x + i) as f32,
                    (cursor_y + j) as f32,
                    0.1,
                    1.0
                ];
                block.build_buffers(&device);
                
                let index;
                if act_slot == 1{
                    game.carpets.push(block);
                    index = game.carpets.len()-1;
                }else{
                    game.decor.push(block);
                    index = game.decor.len()-1;
                }
                block_indices.push(index);
            }
        }
        
        let is_carpet = {
            if act_slot == 1{
                true
            }else{
                false
            }
        };

        // Сохраняем информацию о группе для удаления
        game.groups.push(GroupInfo {
            blocks: block_indices,
            width,
            height,
            pos_x: cursor_x,
            pos_y: cursor_y,
            is_carpet
        });
        
    }
}

pub fn remove(
    game: &mut GameObjects,
){
    let cursor_x = game.cursor.translation[0] as i32;
    let cursor_y = game.cursor.translation[1] as i32;
    
    // Ищем группу, содержащую эту клетку
    let mut group_to_remove = None;
    for (i, group) in game.groups.iter().enumerate() {
        if cursor_x >= group.pos_x && cursor_x < group.pos_x + group.width &&
        cursor_y >= group.pos_y && cursor_y < group.pos_y + group.height {
            group_to_remove = Some(i);
            break;
        }
    }
    
    // Удаляем всю группу
    if let Some(group_index) = group_to_remove {
        let group = game.groups.remove(group_index);
        
        // Удаляем блоки из decor (с конца в начало, чтобы не сбить индексы)
        let mut indices_to_remove = group.blocks;
        indices_to_remove.sort_by(|a, b| b.cmp(a));  // сортируем по убыванию
        
        for index in &indices_to_remove {
            if group.is_carpet{
                game.carpets.remove(*index);
            }else{
                game.decor.remove(*index);
            }
        }
        
        // Обновляем индексы в оставшихся группах
        for remaining_group in &mut game.groups {
            for block_index in &mut remaining_group.blocks {
                let mut shift = 0;
                for removed_index in &indices_to_remove {
                    if *block_index > *removed_index {
                        shift += 1;
                    }
                }
                *block_index -= shift;
            }
        }
    }
}

pub fn can_place_object(game: &GameObjects, x: i32, y: i32, width: i32, height: i32, act_slot: i32) -> bool {
    // Проверка границ
    if x < -4 || x + width > 5 || y < -4 || y + height > 5 {
        return false;
    }
    
    // Проверка, что все клетки свободны
    for i in 0..width {
        for j in 0..height {
            let check_x = x + i;
            let check_y = y + j;
            
            if act_slot == 1{
                for carpet in &game.carpets {
                    let dx = (carpet.translation[0] - check_x as f32).abs();
                    let dy = (carpet.translation[1] - check_y as f32).abs();
                    if dx < 0.5 && dy < 0.5 {
                        return false;
                    }
                }
            }else{
                // Проверяем существующие группы
                for group in &game.groups {
                    if check_x >= group.pos_x && check_x < group.pos_x + group.width &&
                    check_y >= group.pos_y && check_y < group.pos_y + group.height && !group.is_carpet{
                        return false;
                    }
                }
                
                // Проверяем отдельные блоки
                for decor in &game.decor {
                    let dx = (decor.translation[0] - check_x as f32).abs();
                    let dy = (decor.translation[1] - check_y as f32).abs();
                    if dx < 0.5 && dy < 0.5 {
                        return false;
                    }
                }
            }
        }
    }
    
    true
}