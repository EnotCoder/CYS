use specs::{World, WorldExt, Builder, Join};
use std::collections::HashMap;
use crate::Sprite;
use crate::ecs::components::{Transform, SpriteComponent};
use crate::{GroupComponent, GroupInfoResource, GroupInfo};

// ========================================================================
//  SpriteRenderData — плоские данные для рендера (без привязки к ECS)
// ========================================================================
#[derive(Clone)]
pub struct SpriteRenderData {
    pub position: [f32; 3],
    pub texture_path: String,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
}

// ========================================================================
//  EcsAdapter — прослойка между specs ECS и игровой логикой
// ========================================================================
pub struct EcsAdapter {
    pub world: World,
    /// Кеш спрайтов: ключ = "layer_x_y_path_frame_count"
    pub sprite_cache: HashMap<String, Sprite>,
    /// Счётчик group_id (увеличивается)
    pub next_group_id: u32,
    /// Временные сущности превью курсора (показывают размер объекта)
    pub cursor_preview: Vec<specs::Entity>,
}

impl EcsAdapter {
    // ====================================================================
    //  new: Создаёт мир ECS и регистрирует компоненты
    // ====================================================================
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<SpriteComponent>();
        world.register::<GroupComponent>();
        world.insert(GroupInfoResource {
            groups: HashMap::new(),
        });

        Self {
            world,
            sprite_cache: HashMap::new(),
            next_group_id: 1,
            cursor_preview: Vec::new(),
        }
    }

    // ====================================================================
    //  add_cursor: Создаёт курсор (z=2, над decor)
    // ====================================================================
    pub fn add_cursor(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world
            .create_entity()
            .with(Transform {
                position: [x, y, 2.0],
            })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build()
    }

    // ====================================================================
    //  update_cursor_preview: Показывает размер объекта под курсором
    //  Создаёт дополнительные спрайты на каждую клетку занимаемой площади.
    // ====================================================================
    pub fn update_cursor_preview(
        &mut self,
        cursor_x: f32,
        cursor_y: f32,
        width: i32,
        height: i32,
        valid: bool,
    ) {
        self.clear_cursor_preview();

        let tex = if valid { "tex/cursor/cursor.png" } else { "tex/cursor/err cursor.png" };

        for i in 0..width {
            for j in 0..height {
                if i == 0 && j == 0 {
                    continue;
                }
                let entity = self.world
                    .create_entity()
                    .with(Transform {
                        position: [cursor_x + i as f32, cursor_y + j as f32, 2.0],
                    })
                    .with(SpriteComponent {
                        texture_path: tex.to_string(),
                        texture_frame: [0, 0],
                        texture_count: [1, 1],
                    })
                    .build();
                self.cursor_preview.push(entity);
            }
        }
    }

    // ====================================================================
    //  clear_cursor_preview: Удаляет превью-спрайты курсора
    // ====================================================================
    pub fn clear_cursor_preview(&mut self) {
        let entities = self.world.entities();
        let mut transforms = self.world.write_storage::<Transform>();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for &entity in &self.cursor_preview {
            transforms.remove(entity);
            sprites.remove(entity);
            let _ = entities.delete(entity);
        }
        self.cursor_preview.clear();
    }

    // ====================================================================
    //  add_ui: Создаёт элемент UI (z=3, самый верхний слой)
    // ====================================================================
    pub fn add_ui(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world
            .create_entity()
            .with(Transform {
                position: [x, y, 3.0],
            })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build()
    }

    // ====================================================================
    //  get_sprites_by_layer: Группирует все спрайты по слоям (z)
    //
    //  Слои:
    //   0.0 — карта (map)
    //   1.0 — ковры (carpet)
    //   1.5 — декор (decor)
    //   2.0 — курсор (cursor)
    //   3.0 — UI (ui)
    // ====================================================================
    pub fn get_sprites_by_layer(
        &self,
    ) -> (
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
    ) {
        let transforms = self.world.read_storage::<Transform>();
        let sprites = self.world.read_storage::<SpriteComponent>();

        // Заранее выделяем память (обычно ~100-200 спрайтов)
        let mut map_sprites = Vec::with_capacity(100);
        let mut carpet_sprites = Vec::with_capacity(20);
        let mut decor_sprites = Vec::with_capacity(20);
        let mut cursor_sprites = Vec::with_capacity(1);
        let mut ui_sprites = Vec::with_capacity(10);

        for (transform, sprite) in (&transforms, &sprites).join() {
            let data = SpriteRenderData {
                position: transform.position,
                texture_path: sprite.texture_path.clone(),
                texture_frame: sprite.texture_frame,
                texture_count: sprite.texture_count,
            };

            let z = transform.position[2];
            // z: 0.0=map, 1.0=carpet, 1.5=decor, 2.0=cursor, 3.0=ui
            if z == 0.0 {
                map_sprites.push(data);
            } else if z == 1.0 {
                carpet_sprites.push(data);
            } else if z == 1.5 {
                decor_sprites.push(data);
            } else if z == 2.0 {
                cursor_sprites.push(data);
            } else {
                ui_sprites.push(data);
            }
        }

        (
            map_sprites,
            carpet_sprites,
            decor_sprites,
            cursor_sprites,
            ui_sprites,
        )
    }

    // ====================================================================
    //  update_sprite_texture: Меняет текстуру у сущности
    // ====================================================================
    pub fn update_sprite_texture(&mut self, entity: specs::Entity, texture_path: &str) {
        if let Some(sprite) = self
            .world
            .write_storage::<SpriteComponent>()
            .get_mut(entity)
        {
            sprite.texture_path = texture_path.to_string();
        }
    }

    // ====================================================================
    //  update_transform_position: Меняет позицию сущности
    // ====================================================================
    pub fn update_transform_position(&mut self, entity: specs::Entity, x: f32, y: f32) {
        if let Some(transform) = self.world.write_storage::<Transform>().get_mut(entity) {
            transform.position[0] = x;
            transform.position[1] = y;
        }
    }

    // ====================================================================
    //  get_transform_position: Возвращает (x, y) сущности
    // ====================================================================
    pub fn get_transform_position(&self, entity: specs::Entity) -> (f32, f32) {
        self.world
            .read_storage::<Transform>()
            .get(entity)
            .map(|t| (t.position[0], t.position[1]))
            .unwrap_or((0.0, 0.0))
    }

    // ====================================================================
    //  add_group_object: Создаёт групповой объект (несколько сущностей).
    //  Ширина/высота определяют, сколько тайлов занимает объект.
    // ====================================================================
    pub fn add_group_object(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        texture_path: &str,
        base_frame: [i32; 2],
        tex_count: [i32; 2],
        is_carpet: bool,
    ) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let mut entities = Vec::with_capacity((width * height) as usize);
        let z: f32 = if is_carpet { 1.0 } else { 1.5 };

        for i in 0..width {
            for j in 0..height {
                let entity = self
                    .world
                    .create_entity()
                    .with(Transform {
                        position: [(x + i) as f32, (y + j) as f32, z],
                    })
                    .with(SpriteComponent {
                        texture_path: texture_path.to_string(),
                        texture_frame: [base_frame[0] + i, base_frame[1] + j],
                        texture_count: tex_count,
                    })
                    .with(GroupComponent { group_id })
                    .build();
                entities.push(entity);
            }
        }

        // Сохраняем метаданные группы
        self.world
            .write_resource::<GroupInfoResource>()
            .groups
            .insert(
                group_id,
                GroupInfo {
                    entities,
                    width,
                    height,
                    pos_x: x,
                    pos_y: y,
                    is_carpet,
                },
            );

        group_id
    }

    // ====================================================================
    //  delete_group: Удаляет все сущности группы и её метаданные
    // ====================================================================
    pub fn delete_group(&mut self, group_id: u32) {
        let group = self
            .world
            .read_resource::<GroupInfoResource>()
            .groups
            .get(&group_id)
            .cloned();

        if let Some(group) = group {
            // Удаляем каждую сущность группы
            let entities = self.world.entities();
            let mut transforms = self.world.write_storage::<Transform>();
            let mut sprites = self.world.write_storage::<SpriteComponent>();
            let mut group_comps = self.world.write_storage::<GroupComponent>();

            for entity in group.entities {
                let _ = entities.delete(entity);
                transforms.remove(entity);
                sprites.remove(entity);
                group_comps.remove(entity);
            }

            // Удаляем запись группы
            self.world
                .write_resource::<GroupInfoResource>()
                .groups
                .remove(&group_id);
        }
    }

    // ====================================================================
    //  find_group_at_position: Ищет ID группы по координатам сетки
    // ====================================================================
    pub fn find_group_at_position(&self, x: i32, y: i32) -> Option<u32> {
        for (&gid, group) in &self
            .world
            .read_resource::<GroupInfoResource>()
            .groups
        {
            if x >= group.pos_x
                && x < group.pos_x + group.width
                && y >= group.pos_y
                && y < group.pos_y + group.height
            {
                return Some(gid);
            }
        }
        None
    }

    // ====================================================================
    //  can_place_at: Проверяет, можно ли разместить объект.
    //
    //  Правила:
    //   - Объект не должен выходить за границы поля (-4..5)
    //   - Ковёр нельзя ставить на другой ковёр
    //   - Декор можно ставить только на ковёр
    //   - Декор нельзя ставить на другой декор
    // ====================================================================
    pub fn can_place_at(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        is_carpet: bool,
    ) -> bool {
        // Проверка границ: поле 9x9 от -4 до 5
        if x < -4 || x + width > 5 || y < -4 || y + height > 5 {
            return false;
        }

        let transforms = self.world.read_storage::<Transform>();
        let group_comps = self.world.read_storage::<GroupComponent>();
        let groups = &self.world.read_resource::<GroupInfoResource>().groups;

        for i in 0..width {
            for j in 0..height {
                let cx = x + i;
                let cy = y + j;

                for (transform, group_comp) in (&transforms, &group_comps).join() {
                    if transform.position[0] as i32 == cx && transform.position[1] as i32 == cy {
                        if let Some(existing) = groups.get(&group_comp.group_id) {
                            if is_carpet {
                                if existing.is_carpet {
                                    return false; // ковёр на ковёр
                                }
                            } else if !existing.is_carpet {
                                return false; // декор на декор
                            }
                            // Декор на ковёр — OK
                        }
                    }
                }
            }
        }

        true
    }
}
