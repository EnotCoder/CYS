use specs::{World, WorldExt, Builder};
use std::collections::HashMap;
use specs::Join;
use crate::Sprite;
use crate::ecs::components::{Transform, SpriteComponent};
use crate::{GroupComponent, GroupInfoResource};
use crate::constants::Z_UI;

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
    pub sprite_cache: HashMap<String, Sprite>,
    pub next_group_id: u32,
    pub cursor_preview: Vec<specs::Entity>,
}

impl EcsAdapter {
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
    //  Базовые операции над спрайтами
    // ====================================================================

    pub fn update_sprite_texture(&mut self, entity: specs::Entity, texture_path: &str) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.texture_path = texture_path.to_string();
        }
    }

    pub fn update_transform_position(&mut self, entity: specs::Entity, x: f32, y: f32) {
        if let Some(transform) = self.world.write_storage::<Transform>().get_mut(entity) {
            transform.position[0] = x;
            transform.position[1] = y;
        }
    }

    pub fn get_transform_position(&self, entity: specs::Entity) -> (f32, f32) {
        self.world
            .read_storage::<Transform>()
            .get(entity)
            .map(|t| (t.position[0], t.position[1]))
            .unwrap_or((0.0, 0.0))
    }

    // ====================================================================
    //  Создание UI-элементов
    // ====================================================================

    pub fn add_ui(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world
            .create_entity()
            .with(Transform { position: [x, y, Z_UI] })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build()
    }

    pub fn add_ui_sized(
        &mut self,
        x: f32, y: f32,
        width: f32, height: f32,
        texture_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> specs::Entity {
        let entity = self.world
            .create_entity()
            .with(Transform { position: [x, y, Z_UI] })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build();

        let tex = crate::Texture::from_path(device, queue, texture_path, "ui_sized");
        let sprite = crate::Sprite::from_texture(device, &tex, texture_path, width, height);

        let frame_key = format!("{:?}_{:?}", [0, 0], [1, 1]);
        let key = format!("ui_{}_{}_{}_{}", x, y, texture_path, frame_key);
        self.sprite_cache.insert(key, sprite);

        entity
    }

    pub fn add_button(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        text: &str,
        font_size: f32,
        text_renderer: &mut crate::text_renderer::TextRenderer,
    ) -> (specs::Entity, specs::Entity) {
        let bg = self.add_ui_sized(x, y, width, height, "tex/black.png", device, queue);
        let label = text_renderer.add_text(
            self, device, queue,
            text, font_size, x, y + 0.05, width * 0.75, 1.0, [220, 220, 220],
        );
        (bg, label)
    }

    // ====================================================================
    //  Группировка спрайтов по слоям для рендера
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
            if z == crate::constants::Z_MAP {
                map_sprites.push(data);
            } else if z == crate::constants::Z_CARPET {
                carpet_sprites.push(data);
            } else if z == crate::constants::Z_DECOR {
                decor_sprites.push(data);
            } else if z == crate::constants::Z_CURSOR {
                cursor_sprites.push(data);
            } else {
                ui_sprites.push(data);
            }
        }

        (map_sprites, carpet_sprites, decor_sprites, cursor_sprites, ui_sprites)
    }
}
