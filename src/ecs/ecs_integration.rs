use specs::{World, WorldExt, Builder, Join};
use std::collections::HashMap;
use crate::Sprite;
use crate::ecs::components::{Transform, SpriteComponent, GroupMember};
use crate::GroupComponent;
use crate::GroupInfoResource;
use crate::GroupInfo;

// Структура для хранения данных рендера
#[derive(Clone)]
pub struct SpriteRenderData {
    pub position: [f32; 3],
    pub texture_path: String,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub z_index: i32,
}

pub struct EcsAdapter {
    pub world: World,
    pub sprite_cache: HashMap<String, Sprite>,
    pub next_group_id: u32,
}

impl EcsAdapter {
    pub fn new() -> Self {
        let mut world = World::new();
        
        world.register::<Transform>();
        world.register::<SpriteComponent>();
        world.register::<GroupMember>();
        world.register::<GroupComponent>();

        world.insert(GroupInfoResource {
            groups: HashMap::new(),
            next_group_id: 1,
        });
        
        Self {
            world,
            sprite_cache: HashMap::new(),
            next_group_id: 1,
        }
    }
    
    pub fn _add_decor(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world.create_entity()
            .with(Transform {
                position: [x, y, 1.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                z_index: 1,
            })
            .build()
    }
    
    pub fn _add_carpet(&mut self, x: f32, y: f32, width: i32, height: i32, texture_path: &str) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        
        for i in 0..width {
            for j in 0..height {
                self.world.create_entity()
                    .with(Transform {
                        position: [x + i as f32, y + j as f32, 0.0],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    })
                    .with(SpriteComponent {
                        texture_path: texture_path.to_string(),
                        texture_frame: [i, j],
                        texture_count: [width, height],
                        z_index: 0,
                    })
                    .with(GroupMember { group_id })
                    .build();
            }
        }
        
        group_id
    }
    
    pub fn _add_cursor(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world.create_entity()
            .with(Transform {
                position: [x, y, 2.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                z_index: 2,
            })
            .build()
    }
    
    pub fn _add_ui(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world.create_entity()
            .with(Transform {
                position: [x, y, 3.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                z_index: 3,
            })
            .build()
    }
    
    pub fn get_sprites_by_layer(&self) -> (Vec<SpriteRenderData>, Vec<SpriteRenderData>, Vec<SpriteRenderData>, Vec<SpriteRenderData>) {
        let transforms = self.world.read_storage::<Transform>();
        let sprites = self.world.read_storage::<SpriteComponent>();
        
        let mut map_sprites = Vec::new();
        let mut decor_sprites = Vec::new();
        let mut cursor_sprites = Vec::new();
        let mut ui_sprites = Vec::new();
        
        for (transform, sprite) in (&transforms, &sprites).join() {
            let data = SpriteRenderData {
                position: transform.position,
                texture_path: sprite.texture_path.clone(),
                texture_frame: sprite.texture_frame,
                texture_count: sprite.texture_count,
                z_index: sprite.z_index,
            };
            
            let z = transform.position[2] as i32;
            match z {
                0 => map_sprites.push(data),
                1 => decor_sprites.push(data),
                2 => cursor_sprites.push(data),
                3 => ui_sprites.push(data),
                _ => decor_sprites.push(data),
            }
        }
        
        (map_sprites, decor_sprites, cursor_sprites, ui_sprites)
    }
    
    pub fn update_sprite_texture(&mut self, entity: specs::Entity, texture_path: &str) {
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        if let Some(sprite) = sprites.get_mut(entity) {
            sprite.texture_path = texture_path.to_string();
        }
    }
    
    pub fn update_transform_position(&mut self, entity: specs::Entity, x: f32, y: f32) {
        let mut transforms = self.world.write_storage::<Transform>();
        if let Some(transform) = transforms.get_mut(entity) {
            transform.position[0] = x;
            transform.position[1] = y;
        }
    }
    
    pub fn get_transform_position(&self, entity: specs::Entity) -> (f32, f32) {
        let transforms = self.world.read_storage::<Transform>();
        if let Some(transform) = transforms.get(entity) {
            (transform.position[0], transform.position[1])
        } else {
            (0.0, 0.0)
        }
    }

    // Добавляем групповой объект
    pub fn add_group_object(
        &mut self,
        x: f32,
        y: f32,
        width: i32,
        height: i32,
        texture_path: &str,
        texture_base_frame: [i32; 2],
        texture_count: [i32; 2],
        is_carpet: bool,
    ) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        
        let mut entities = Vec::new();
        let z = if is_carpet { 0.0 } else { 1.0 };
        let z_index = if is_carpet { 0 } else { 1 };
        
        for i in 0..width {
            for j in 0..height {
                let entity = self.world.create_entity()
                    .with(Transform {
                        position: [x + i as f32, y + j as f32, z],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        scale: [1.0, 1.0, 1.0],
                    })
                    .with(SpriteComponent {
                        texture_path: texture_path.to_string(),
                        texture_frame: [texture_base_frame[0] + i, texture_base_frame[1] + j],
                        texture_count,
                        z_index,
                    })
                    .with(GroupComponent {
                        group_id,
                        is_leader: i == 0 && j == 0, // первый блок - лидер
                    })
                    .build();
                entities.push(entity);
            }
        }
        
        // Сохраняем информацию о группе
        let mut groups_resource = self.world.write_resource::<GroupInfoResource>();
        groups_resource.groups.insert(group_id, GroupInfo {
            entities,
            width,
            height,
            pos_x: x as i32,
            pos_y: y as i32,
            is_carpet,
        });
        
        group_id
    }
    
    // Удаляем группу по ID
    pub fn delete_group(&mut self, group_id: u32) {
        // Сначала получаем копию информации о группе (только чтение)
        let group_to_delete = {
            let groups_resource = self.world.read_resource::<GroupInfoResource>();
            groups_resource.groups.get(&group_id).cloned()
        };
        
        if let Some(group) = group_to_delete {
            // Удаляем все сущности группы
            let mut entities = self.world.entities();
            let mut transforms = self.world.write_storage::<Transform>();
            let mut sprites = self.world.write_storage::<SpriteComponent>();
            let mut group_components = self.world.write_storage::<GroupComponent>();
            
            for entity in group.entities {
                let _ = entities.delete(entity);
                transforms.remove(entity);
                sprites.remove(entity);
                group_components.remove(entity);
            }
            
            // Теперь отдельно удаляем информацию о группе (новая область видимости)
            let mut groups_resource = self.world.write_resource::<GroupInfoResource>();
            groups_resource.groups.remove(&group_id);
        }
    }
    
    // Находим группу по позиции курсора
    pub fn find_group_at_position(&self, x: i32, y: i32) -> Option<u32> {
        let groups_resource = self.world.read_resource::<GroupInfoResource>();
        
        for (group_id, group) in &groups_resource.groups {
            if x >= group.pos_x && x < group.pos_x + group.width &&
               y >= group.pos_y && y < group.pos_y + group.height {
                return Some(*group_id);
            }
        }
        None
    }
    
    // Проверка возможности размещения объекта
    pub fn can_place_at(&self, x: i32, y: i32, width: i32, height: i32, is_carpet: bool) -> bool {
        // Проверка границ
        if x < -4 || x + width > 5 || y < -4 || y + height > 5 {
            return false;
        }
        
        let transforms = self.world.read_storage::<Transform>();
        let group_components = self.world.read_storage::<GroupComponent>();
        let groups_resource = self.world.read_resource::<GroupInfoResource>();
        
        for i in 0..width {
            for j in 0..height {
                let check_x = x + i;
                let check_y = y + j;
                
                for (transform, group_comp) in (&transforms, &group_components).join() {
                    let dx = (transform.position[0] - check_x as f32).abs();
                    let dy = (transform.position[1] - check_y as f32).abs();
                    
                    if dx < 0.5 && dy < 0.5 {
                        if is_carpet {
                            // Ковры могут перекрывать только другие ковры?
                            if let Some(group) = groups_resource.groups.get(&group_comp.group_id) {
                                if group.is_carpet {
                                    return false;
                                }
                            }
                        } else {
                            // Декор не может перекрывать другой декор
                            return false;
                        }
                    }
                }
            }
        }
        
        true
    }
}