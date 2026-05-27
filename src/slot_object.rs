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
}

pub struct GameObjects {
    pub cursor: Sprite,
    pub map: Vec<Sprite>,
    pub decor: Vec<Sprite>,
    pub groups: Vec<GroupInfo>,
}

pub struct Object{
    pub sprite: Sprite,
    pub width: i32,
    pub height: i32,
    pub name: String,
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
    if can_place_object(&game, cursor_x, cursor_y, width, height) {
        let mut block_indices = Vec::new();
        
        // Создаём блоки для каждой клетки
        for i in 0..width {
            for j in 0..height {
                let mut block = Sprite::new(
                    &device, &queue,
                    &active_slot.obj.sprite.texture_path,
                    active_slot.obj.sprite.texture_frame,
                    active_slot.obj.sprite.texture_count,
                );
                block.translation = [
                    (cursor_x + i) as f32,
                    (cursor_y + j) as f32,
                    0.1,
                    1.0
                ];
                block.build_buffers(&device);
                
                let index = game.decor.len();
                game.decor.push(block);
                block_indices.push(index);
            }
        }
        
        // Сохраняем информацию о группе для удаления
        game.groups.push(GroupInfo {
            blocks: block_indices,
            width,
            height,
            pos_x: cursor_x,
            pos_y: cursor_y,
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
            game.decor.remove(*index);
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

pub fn can_place_object(game: &GameObjects, x: i32, y: i32, width: i32, height: i32) -> bool {
    // Проверка границ
    if x < -4 || x + width > 5 || y < -4 || y + height > 5 {
        return false;
    }
    
    // Проверка, что все клетки свободны
    for i in 0..width {
        for j in 0..height {
            let check_x = x + i;
            let check_y = y + j;
            
            // Проверяем существующие группы
            for group in &game.groups {
                if check_x >= group.pos_x && check_x < group.pos_x + group.width &&
                   check_y >= group.pos_y && check_y < group.pos_y + group.height {
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
    
    true
}