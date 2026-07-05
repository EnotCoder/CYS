pub mod render;

use specs::{World, WorldExt};
use std::collections::{HashMap, HashSet};
use crate::Sprite;
use crate::ecs::components::{
    FenceComponent, FoodStorage, Money, ObjectTag, Rotation,
    SpriteComponent, TotalFood, Transform, BusyCassas,
};
use crate::{GroupComponent, GroupInfoResource};
use crate::constants::*;
use crate::util;

// ========================================================================
//  SpriteRenderData — плоские данные для рендера (без привязки к ECS)
// ========================================================================
#[derive(Clone)]
pub struct SpriteRenderData {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub texture_path: String,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub scale: f32,
    pub alpha: f32,
}

// ========================================================================
//  EcsAdapter — прослойка между specs ECS и игровой логикой
// ========================================================================
pub struct EcsAdapter {
    pub world: World,
    pub sprite_cache: HashMap<u64, Sprite>,
    pub next_group_id: u32,
    pub cursor_preview: Vec<specs::Entity>,
    pub wall_positions: HashSet<(i32, i32)>,
    pub floor_positions: HashSet<(i32, i32)>,
    pub outdoor_positions: HashSet<(i32, i32)>,
    pub flower_positions: HashSet<(i32, i32)>,
    pub floor_placed_positions: HashSet<(i32, i32)>,
    pub map_grid: Vec<Vec<String>>,
    pub map_entities: HashMap<(i32, i32), specs::Entity>,
    pub original_tokens: HashMap<(i32, i32), String>,
}

impl EcsAdapter {
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<SpriteComponent>();
        world.register::<GroupComponent>();
        world.register::<Rotation>();
        world.register::<ObjectTag>();
        world.register::<FoodStorage>();
        world.register::<FenceComponent>();
        world.insert(GroupInfoResource {
            groups: HashMap::new(),
        });
        world.insert(TotalFood(0));
        world.insert(BusyCassas(HashSet::new()));
        world.insert(Money(0));

        Self {
            world,
            sprite_cache: HashMap::new(),
            next_group_id: 1,
            cursor_preview: Vec::new(),
            wall_positions: HashSet::new(),
            floor_positions: HashSet::new(),
            outdoor_positions: HashSet::new(),
            flower_positions: HashSet::new(),
            floor_placed_positions: HashSet::new(),
            map_grid: Vec::new(),
            map_entities: HashMap::new(),
            original_tokens: HashMap::new(),
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

    pub fn update_sprite_alpha(&mut self, entity: specs::Entity, alpha: f32) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.alpha = alpha;
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
    //  Удаление entity
    // ====================================================================

    pub fn delete_entity(&self, entity: specs::Entity) {
        let _ = self.world.entities().delete(entity);
        self.world.write_storage::<Transform>().remove(entity);
        self.world.write_storage::<SpriteComponent>().remove(entity);
    }

    pub fn delete_entities(&self, entities: &[specs::Entity]) {
        for &ent in entities {
            self.delete_entity(ent);
        }
    }

    // ====================================================================
    //  Создание UI-элементов
    // ====================================================================

    pub fn add_ui(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_UI,
            texture_path, [0, 0], [1, 1], 1.0, 1.0,
        )
    }

    pub fn add_ui_sized(
        &mut self,
        x: f32, y: f32,
        width: f32, height: f32,
        texture_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> specs::Entity {
        let entity = crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_UI,
            texture_path, [0, 0], [1, 1], 1.0, 1.0,
        );

        let tex = crate::Texture::from_path(device, queue, texture_path, "ui_sized");
        let sprite = crate::Sprite::from_texture(device, &tex, texture_path, width, height);

        let key = util::sprite_cache_key("ui", x, y, texture_path, [0, 0], [1, 1], 1.0);
        self.sprite_cache.insert(key, sprite);

        entity
    }

    pub fn save_map_grid(&self) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::File::create(crate::constants::MAP_FILE) {
            for row in &self.map_grid {
                let line = row.join(" ");
                let _ = writeln!(file, "{}", line);
            }
        }
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
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
    ) -> (specs::Entity, specs::Entity) {
        let bg = self.add_ui_sized(x, y, width, height, "tex/dev_tools/black.png", device, queue);
        let label = text_renderer.add_text(
            self, device, queue,
            text, font_size, x, y + 0.05, width * 0.75, 1.0, BTN_TEXT_COLOR,
        );
        (bg, label)
    }

}
