use specs::{World, WorldExt, Join};
use std::collections::{HashMap, HashSet};
use crate::Sprite;
use crate::ecs::components::{Transform, SpriteComponent, Rotation, ObjectTag, FoodStorage, TotalFood, FenceComponent, BusyCassas, Money};
use crate::{GroupComponent, GroupInfoResource};
use crate::constants::*;

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
    pub sprite_cache: HashMap<String, Sprite>,
    pub next_group_id: u32,
    pub cursor_preview: Vec<specs::Entity>,
    pub wall_positions: HashSet<(i32, i32)>,
    pub floor_positions: HashSet<(i32, i32)>,
    pub outdoor_positions: HashSet<(i32, i32)>,
    pub flower_positions: HashSet<(i32, i32)>,
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

        let frame_key = format!("{:?}_{:?}", [0, 0], [1, 1]);
        let key = format!("ui_{}_{}_{}_{}_1", x, y, texture_path, frame_key);
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
            text, font_size, x, y + 0.05, width * 0.75, 1.0, BTN_TEXT_COLOR,
        );
        (bg, label)
    }

    // ====================================================================
    //  Группировка спрайтов по слоям для рендера
    // ====================================================================

    pub fn get_sprites_by_layer(
        &self,
        visible_bounds: Option<(f32, f32, f32, f32)>,
    ) -> (
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
    ) {
        let transforms = self.world.read_storage::<Transform>();
        let sprites = self.world.read_storage::<SpriteComponent>();
        let rotations = self.world.read_storage::<Rotation>();

        let margin = 2.0;
        let mut map_sprites = Vec::with_capacity(100);
        let mut carpet_sprites = Vec::with_capacity(20);
        let mut decor_sprites = Vec::with_capacity(20);
        let mut npc_sprites = Vec::with_capacity(5);
        let mut cursor_sprites = Vec::with_capacity(1);
        let mut ui_sprites = Vec::with_capacity(10);

        for (transform, sprite, rotation_opt) in (&transforms, &sprites, rotations.maybe()).join() {
            let data = SpriteRenderData {
                position: transform.position,
                rotation: rotation_opt.map(|r| r.rotation).unwrap_or([0.0; 3]),
                texture_path: sprite.texture_path.clone(),
                texture_frame: sprite.texture_frame,
                texture_count: sprite.texture_count,
                scale: sprite.scale,
                alpha: sprite.alpha,
            };

            let z = transform.position[2];
            let should_cull = z == crate::constants::Z_MAP
                || z == crate::constants::Z_CARPET
                || z == crate::constants::Z_DECOR
                || z == crate::constants::Z_NPC;
            if should_cull {
                if let Some((l, r, b, t)) = visible_bounds {
                    let x = transform.position[0];
                    let y = transform.position[1];
                    if x + 1.0 + margin < l || x - margin > r || y + 1.0 + margin < b || y - margin > t {
                        continue;
                    }
                }
            }
            if z == crate::constants::Z_MAP {
                map_sprites.push(data);
            } else if z == crate::constants::Z_CARPET {
                carpet_sprites.push(data);
            } else if z == crate::constants::Z_DECOR {
                decor_sprites.push(data);
            } else if z == crate::constants::Z_NPC {
                npc_sprites.push(data);
            } else if z == crate::constants::Z_CURSOR {
                cursor_sprites.push(data);
            } else {
                ui_sprites.push(data);
            }
        }

        (map_sprites, carpet_sprites, decor_sprites, npc_sprites, cursor_sprites, ui_sprites)
    }

    pub fn update_object_textures(&mut self) {
        let mut updates: Vec<(u32, String)> = Vec::new();
        {
            let tags = self.world.read_storage::<ObjectTag>();
            let foods = self.world.read_storage::<FoodStorage>();
            let groups = self.world.read_storage::<GroupComponent>();
            for (tag, food, group) in (&tags, &foods, &groups).join() {
                let tex = if tag.name == "box" {
                    if food.food_count < 8 {
                        "tex/decor/box/box_0.png"
                    } else if food.food_count < 12 {
                        "tex/decor/box/box_1.png"
                    } else {
                        "tex/decor/box/box_2.png"
                    }
                } else if tag.name == "rack" {
                    if food.food_count == 0 {
                        "tex/decor/rack/rack_0.png"
                    } else {
                        "tex/decor/rack/rack_1.png"
                    }
                } else {
                    continue;
                };
                updates.push((group.group_id, tex.to_string()));
            }
        }
        let group_info = self.world.read_resource::<crate::GroupInfoResource>();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (gid, tex) in &updates {
            if let Some(info) = group_info.groups.get(gid) {
                for &entity in &info.entities {
                    if let Some(sprite) = sprites.get_mut(entity) {
                        sprite.texture_path = tex.clone();
                    }
                }
            }
        }
    }

    pub fn update_fence_textures(&mut self) {
        use std::path::Path;
        let transforms = self.world.read_storage::<Transform>();
        let fences = self.world.read_storage::<FenceComponent>();
        let positions: HashSet<(i32, i32)> = (&fences, &transforms)
            .join()
            .map(|(_, t)| (t.position[0] as i32, t.position[1] as i32))
            .collect();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (fence, transform, sprite) in (&fences, &transforms, &mut sprites).join() {
            let x = transform.position[0] as i32;
            let y = transform.position[1] as i32;
            let right = positions.contains(&(x + 1, y));
            let left = positions.contains(&(x - 1, y));
            let up = positions.contains(&(x, y + 1));
            let down = positions.contains(&(x, y - 1));
            let (dir, fallback) = if fence.name == "street_fence" {
                ("tex/decor/street_fence/street_fence", "tex/decor/street_fence/street_fence_0_0_0_0.png")
            } else {
                ("tex/decor/fence/fence", "tex/decor/fence/fence_0_0_0_0.png")
            };
            let path = format!("{}_{}_{}_{}_{}.png", dir, up as u8, down as u8, left as u8, right as u8);
            if Path::new(&path).exists() {
                sprite.texture_path = path;
            } else {
                sprite.texture_path = fallback.to_string();
            }
        }
    }
}
