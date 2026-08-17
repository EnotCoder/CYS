// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  EcsAdapter — прослойка между specs-миром и игрой. Здесь создаются:
//  EcsAdapter (world, кэши спрайтов/текстур, карта, позиционные множества),
//  SpriteRenderData (плоские данные для рендера), операции над спрайтами,
//  создание UI-элементов и очистка мира.
// ========================================================================

pub mod render;

use specs::{World, WorldExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::Sprite;
use crate::Texture;
use crate::ecs::components::{
    BasementPlaced, FenceComponent, FoodStorage, Money, ObjectTag, PointLight, Rotation,
    SpriteComponent, TotalFood, Transform, BusyCassas,
};
use crate::{GroupComponent, GroupInfoResource};
use crate::core::constants::*;
use crate::core::util;

// ========================================================================
//  SpriteRenderData — плоские данные для рендера (без привязки к ECS)
// ========================================================================
// Система рендера получает их из get_sprites_by_layer и не знает о внутреннем
// устройстве ECS-мира.
#[derive(Clone)]
pub struct SpriteRenderData {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub texture_path: Arc<str>,
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
    // Кэш готовых к отрисовке спрайтов (ключ = sprite_cache_key:
    // слой + путь + кадр + атлас + масштаб) — избегает пересоздания GPU-ресурсов.
    pub sprite_cache: HashMap<u64, Sprite>,
    // Кэш загруженных текстур по базовому пути (без суффикса «@WxH»),
    // чтобы анимации масштаба не перечитывали файлы и не дублировали текстуры.
    pub texture_cache: HashMap<String, Texture>,
    // Инкрементальный счётчик ID многоклеточных объектов (групп).
    pub next_group_id: u32,
    pub current_level: i32,
    // Счётчик полных очисток мира (используется для сброса кэшей/состояний).
    pub clear_count: u64,
    // Временные сущности призрака размещения — пересоздаются каждый кадр.
    pub cursor_preview: Vec<specs::Entity>,
    // Очередь событий «в объект добавлена еда»: id групп объектов.
    // Наполняется ящиком-регеном и пополнением стеллажа, дренится сценой для анимаций.
    pub pending_food_adds: Vec<u32>,
    // Наборы клеток карты, разрешённых для размещения каждой категории
    // объектов (вычисляются при загрузке карты).
    pub wall_positions: HashSet<(i32, i32)>,
    pub floor_positions: HashSet<(i32, i32)>,
    pub outdoor_positions: HashSet<(i32, i32)>,
    pub flower_positions: HashSet<(i32, i32)>,
    // Клетки пола в текущей постройке и клетки, куда пол ещё можно положить.
    pub floor_placed_positions: HashSet<(i32, i32)>,
    pub floor_placeable_positions: HashSet<(i32, i32)>,
    // Текущая сетка карты, соответствие клетка -> сущность тайла, а также
    // исходные токены для восстановления карты после стирания объектов.
    pub map_grid: Vec<Vec<String>>,
    pub map_entities: HashMap<(i32, i32), specs::Entity>,
    pub original_tokens: HashMap<(i32, i32), String>,
}

impl EcsAdapter {
    pub fn new() -> Self {
        let mut world = World::new();
        // Регистрация всех компонентов и игровых ресурсов.
        world.register::<Transform>();
        world.register::<SpriteComponent>();
        world.register::<GroupComponent>();
        world.register::<Rotation>();
        world.register::<ObjectTag>();
        world.register::<FoodStorage>();
        world.register::<FenceComponent>();
        world.register::<PointLight>();
        world.insert(GroupInfoResource {
            groups: HashMap::new(),
        });
        world.insert(TotalFood(0));
        world.insert(BusyCassas(HashSet::new()));
        let cfg = crate::scripts::config::BalanceConfig::load();
        world.insert(Money(cfg.start_money));
        world.insert(BasementPlaced(false));
        world.insert(cfg);

        Self {
            world,
            sprite_cache: HashMap::new(),
            texture_cache: HashMap::new(),
            next_group_id: 1,
            current_level: 0,
            clear_count: 0,
            cursor_preview: Vec::new(),
            pending_food_adds: Vec::new(),
            wall_positions: HashSet::new(),
            floor_positions: HashSet::new(),
            outdoor_positions: HashSet::new(),
            flower_positions: HashSet::new(),
            floor_placed_positions: HashSet::new(),
            floor_placeable_positions: HashSet::new(),
            map_grid: Vec::new(),
            map_entities: HashMap::new(),
            original_tokens: HashMap::new(),
        }
    }

    // ====================================================================
    //  Базовые операции над спрайтами
    // ====================================================================

    // Меняет текстуру существующего спрайта (смена состояния, анимация).
    pub fn update_sprite_texture(&mut self, entity: specs::Entity, texture_path: &str) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.texture_path = Arc::from(texture_path);
        }
    }

    // Передвигает сущность на новую позицию в мировых координатах.
    pub fn update_transform_position(&mut self, entity: specs::Entity, x: f32, y: f32) {
        if let Some(transform) = self.world.write_storage::<Transform>().get_mut(entity) {
            transform.position[0] = x;
            transform.position[1] = y;
        }
    }

    // Меняет прозрачность спрайта (моргание, эффекты выделения).
    pub fn update_sprite_alpha(&mut self, entity: specs::Entity, alpha: f32) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.alpha = alpha;
        }
    }

    // Меняет масштаб отдельного спрайта (UI-анимации: пульс, поп-эффекты).
    pub fn update_sprite_scale(&mut self, entity: specs::Entity, scale: f32) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.scale = scale;
        }
    }

    // Меняет масштаб всех спрайтов группы (эффект «поп» при пополнении).
    // Если группы уже нет — безопасно делает ничего.
    pub fn update_group_scale(&mut self, group_id: u32, scale: f32) {
        let group_info = self.world.read_resource::<GroupInfoResource>();
        let Some(info) = group_info.groups.get(&group_id) else { return };
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for &entity in &info.entities {
            if let Some(sprite) = sprites.get_mut(entity) {
                sprite.scale = scale;
            }
        }
    }

    // Возвращает (x, y) сущности или (0,0), если компонента нет.
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

    // Удаляет сущность из реестра и из хранилищ её компонентов.
    pub fn delete_entity(&self, entity: specs::Entity) {
        let _ = self.world.entities().delete(entity);
        self.world.write_storage::<Transform>().remove(entity);
        self.world.write_storage::<SpriteComponent>().remove(entity);
    }

    // Удаляет пачку сущностей (используется для предпросмотра курсора).
    pub fn delete_entities(&self, entities: &[specs::Entity]) {
        for &ent in entities {
            self.delete_entity(ent);
        }
    }

    // ====================================================================
    //  Создание UI-элементов
    // ====================================================================

    // Спрайт интерфейса без масштабирования (размер определяет текстура).
    pub fn add_ui(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_UI,
            texture_path, [0, 0], [1, 1], 1.0, 1.0,
        )
    }

    // UI-спрайт с явным размером width×height. Ключ кэша строится из
    // текстуры + размера ("path@WxH"), чтобы одинаковые элементы
    // не пересоздавали quad каждый раз.
    pub fn add_ui_sized(
        &mut self,
        x: f32, y: f32,
        width: f32, height: f32,
        texture_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> specs::Entity {
        // Уникальное имя: путь + размер — совпадает с ключом кэша спрайтов.
        let unique = format!("{}@{:.2}x{:.2}", texture_path, width, height);

        let entity = crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_UI,
            &unique, [0, 0], [1, 1], 1.0, 1.0,
        );

        // Готовим quad нужного размера и кладём в кэш по тому же имени.
        let tex = crate::Texture::from_path(device, queue, texture_path, "ui_sized");
        let sprite = crate::Sprite::from_texture(device, &tex, &unique, width, height);

        let key = util::sprite_cache_key("ui", &unique, [0, 0], [1, 1], 1.0);
        self.sprite_cache.insert(key, sprite);

        entity
    }

    // Полная очистка мира между уровнями: удаляет все сущности,
    // сбрасывает кэши, счётчики, карту и ресурсы до начального состояния.
    pub fn clear_world(&mut self) {
        use specs::Join;
        let delete_entities: Vec<specs::Entity> = {
            let entities = self.world.entities();
            let transforms = self.world.read_storage::<Transform>();
            let sprites = self.world.read_storage::<SpriteComponent>();
            (&entities, &transforms, &sprites).join()
                .map(|(e, _, _)| e)
                .collect()
        };
        {
            let mut transforms = self.world.write_storage::<Transform>();
            let mut sprites = self.world.write_storage::<SpriteComponent>();
            let mut group_comps = self.world.write_storage::<GroupComponent>();
            let mut lights = self.world.write_storage::<PointLight>();
            let entities = self.world.entities();
            for e in delete_entities {
                transforms.remove(e);
                sprites.remove(e);
                group_comps.remove(e);
                lights.remove(e);
                let _ = entities.delete(e);
            }
        }
        self.sprite_cache.clear();
        self.texture_cache.clear();
        self.cursor_preview.clear();
        self.pending_food_adds.clear();
        self.next_group_id = 1;
        self.clear_count += 1;
        self.map_grid.clear();
        self.map_entities.clear();
        self.original_tokens.clear();
        self.wall_positions.clear();
        self.floor_positions.clear();
        self.outdoor_positions.clear();
        self.flower_positions.clear();
        self.floor_placed_positions.clear();
        self.floor_placeable_positions.clear();
        self.world.write_resource::<crate::GroupInfoResource>().groups.clear();
        self.world.write_resource::<BasementPlaced>().0 = false;
    }

}
