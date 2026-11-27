// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  EcsAdapter — прослойка между specs-миром и игрой. Здесь создаются:
//  EcsAdapter (world, кэши спрайтов/текстур, карта, позиционные множества),
//  SpriteRenderData (плоские данные для рендера), операции над спрайтами,
//  создание UI-элементов и очистка мира.
// ========================================================================

pub mod render;

use crate::core::constants::*;
use crate::core::util;
use crate::ecs::components::{
    BasementPlaced, BusyCassas, DoorProgress, FenceComponent, FoodStorage, Money, ObjectTag,
    PlacementError, PointLight, Rotation, Season, ShopDenied, ShopOwned, ShopperTag,
    SpriteComponent, TotalFood, Transform,
};
use crate::ecs::particles::PlacementFxEvent;
use crate::Sprite;
use crate::Texture;
use crate::{GroupComponent, GroupInfoResource};
use specs::{World, WorldExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    // Путь текстуры забора по (тип, маска соседей, версия пака/модов):
    // избавляет от проверок файла на диске каждый кадр.
    fence_paths: HashMap<(bool, u8, u64), Arc<str>>,
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
    // Очередь событий установки/удаления объекта: дренится сценой для партиклов.
    pub pending_placement_fx: Vec<PlacementFxEvent>,
    // Наборы клеток карты, разрешённых для размещения каждой категории
    // объектов (вычисляются при загрузке карты).
    pub wall_positions: HashSet<(i32, i32)>,
    pub floor_positions: HashSet<(i32, i32)>,
    pub outdoor_positions: HashSet<(i32, i32)>,
    pub flower_positions: HashSet<(i32, i32)>,
    // Текущая сетка карты, соответствие клетка -> сущность тайла, а также
    // исходные токены для восстановления карты после стирания объектов.
    pub map_grid: Vec<Vec<String>>,
    pub map_entities: HashMap<(i32, i32), specs::Entity>,
    pub original_tokens: HashMap<(i32, i32), String>,
    // Один спрайт на всю карту вместо отдельного спрайта на каждую клетку.
    pub map_background: Option<specs::Entity>,
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
        world.register::<ShopperTag>();
        world.insert(GroupInfoResource {
            groups: HashMap::new(),
        });
        world.insert(TotalFood(0));
        world.insert(BusyCassas(HashSet::new()));
        world.insert(DoorProgress::new());
        let cfg = crate::scripts::config::BalanceConfig::load();
        world.insert(Money(cfg.start_money));
        world.insert(BasementPlaced(false));
        world.insert(PlacementError(None));
        world.insert(ShopOwned(Vec::new()));
        world.insert(ShopDenied(false));
        world.insert(Season::default());
        world.insert(cfg);

        Self {
            world,
            sprite_cache: HashMap::new(),
            texture_cache: HashMap::new(),
            fence_paths: HashMap::new(),
            next_group_id: 1,
            current_level: 0,
            clear_count: 0,
            cursor_preview: Vec::new(),
            pending_food_adds: Vec::new(),
            pending_placement_fx: Vec::new(),
            wall_positions: HashSet::new(),
            floor_positions: HashSet::new(),
            outdoor_positions: HashSet::new(),
            flower_positions: HashSet::new(),
            map_grid: Vec::new(),
            map_entities: HashMap::new(),
            original_tokens: HashMap::new(),
            map_background: None,
        }
    }

    // ====================================================================
    //  Базовые операции над спрайтами
    // ====================================================================

    // Меняет текстуру существующего спрайта (смена состояния, анимация).
    pub fn update_sprite_texture(&mut self, entity: specs::Entity, texture_path: &str) {
        if let Some(sprite) = self
            .world
            .write_storage::<SpriteComponent>()
            .get_mut(entity)
        {
            sprite.texture_path = Arc::from(texture_path);
        }
    }

    // Возвращает true, если на текстуру ещё ссылается живая сущность, кроме
    // `except`. Одинаковые подписи (например, статусы двух модов) делят один
    // спрайт в кэше, поэтому удалять его можно только когда он нигде не виден.
    pub fn texture_path_in_use(&self, texture_path: &str, except: Option<specs::Entity>) -> bool {
        use specs::Join;
        let sprites = self.world.read_storage::<SpriteComponent>();
        let entities = self.world.entities();
        (&entities, &sprites).join().any(|(entity, sprite)| {
            Some(entity) != except && sprite.texture_path.as_ref() == texture_path
        })
    }

    // Убирает спрайт из кэша, только если его текстура больше не используется.
    // Иначе рендер не найдёт спрайт и подставит текстуру-заглушку null.png.
    pub fn remove_sprite_if_unused(
        &mut self,
        key: u64,
        texture_path: &str,
        except: Option<specs::Entity>,
    ) -> bool {
        if self.texture_path_in_use(texture_path, except) {
            return false;
        }
        self.sprite_cache.remove(&key);
        true
    }

    // Передвигает сущность на новую позицию в мировых координатах.
    pub fn update_transform_position(&mut self, entity: specs::Entity, x: f32, y: f32) {
        if let Some(transform) = self.world.write_storage::<Transform>().get_mut(entity) {
            transform.position[0] = x;
            transform.position[1] = y;
        }
    }

    // Меняет z-координату (слой) сущности.
    pub fn update_transform_z(&mut self, entity: specs::Entity, z: f32) {
        if let Some(transform) = self.world.write_storage::<Transform>().get_mut(entity) {
            transform.position[2] = z;
        }
    }

    // Меняет прозрачность спрайта (моргание, эффекты выделения).
    pub fn update_sprite_alpha(&mut self, entity: specs::Entity, alpha: f32) {
        if let Some(sprite) = self
            .world
            .write_storage::<SpriteComponent>()
            .get_mut(entity)
        {
            sprite.alpha = alpha;
        }
    }

    // Меняет масштаб отдельного спрайта (UI-анимации: пульс, поп-эффекты).
    pub fn update_sprite_scale(&mut self, entity: specs::Entity, scale: f32) {
        if let Some(sprite) = self
            .world
            .write_storage::<SpriteComponent>()
            .get_mut(entity)
        {
            sprite.scale = scale;
        }
    }

    // Меняет масштаб всех спрайтов группы (эффект «поп» при пополнении).
    // Если группы уже нет — безопасно делает ничего.
    pub fn update_group_scale(&mut self, group_id: u32, scale: f32) {
        let group_info = self.world.read_resource::<GroupInfoResource>();
        let Some(info) = group_info.groups.get(&group_id) else {
            return;
        };
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
            &mut self.world,
            x,
            y,
            Z_UI,
            texture_path,
            [0, 0],
            [1, 1],
            1.0,
            1.0,
        )
    }

    // UI-спрайт с явным размером width×height. Ключ кэша строится из
    // текстуры + размера ("path@WxH"), чтобы одинаковые элементы
    // не пересоздавали quad каждый раз.
    pub fn add_ui_sized(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> specs::Entity {
        // Уникальное имя: путь + размер — совпадает с ключом кэша спрайтов.
        let unique = format!("{}@{:.2}x{:.2}", texture_path, width, height);

        let entity = crate::ecs::factory::create_sprite(
            &mut self.world,
            x,
            y,
            Z_UI,
            &unique,
            [0, 0],
            [1, 1],
            1.0,
            1.0,
        );

        // Готовим quad нужного размера и кладём в кэш по тому же имени.
        let tex = crate::Texture::from_path(device, queue, texture_path, "ui_sized");
        let sprite = crate::Sprite::from_texture(device, &tex, &unique, width, height);

        let key = util::sprite_cache_key("ui", &unique, [0, 0], [1, 1], 1.0);
        self.sprite_cache.insert(key, sprite);

        entity
    }

    /// Применяет новый текстур-пак к уже созданным спрайтам, не убивая уровень.
    /// Кэш чистится выборочно: сгенерированные текстом и погодой спрайты
    /// сохраняются, иначе интерфейс и осадки пропали бы до пересоздания.
    pub fn apply_texture_pack(&mut self) {
        use specs::Join;

        let mut keep = HashSet::new();
        {
            let transforms = self.world.read_storage::<Transform>();
            let sprites = self.world.read_storage::<SpriteComponent>();
            for (transform, sprite) in (&transforms, &sprites).join() {
                if sprite.texture_path.starts_with("assets/") {
                    continue;
                }
                let prefix = if transform.position[2] == Z_MAP {
                    "map"
                } else if transform.position[2] < Z_UI {
                    "transparent"
                } else {
                    "ui"
                };
                keep.insert(util::sprite_cache_key(
                    prefix,
                    &sprite.texture_path,
                    sprite.texture_frame,
                    sprite.texture_count,
                    sprite.scale,
                ));
            }
        }
        self.sprite_cache.retain(|key, _| keep.contains(key));
        self.texture_cache.clear();
        self.fence_paths.clear();
        crate::core::texture_pack::clear_caches();
        crate::core::jobs::clear_decoded();
        crate::core::jobs::prewarm_game_textures();

        let groups = self
            .world
            .read_resource::<GroupInfoResource>()
            .groups
            .clone();
        let mut changes: Vec<(
            specs::Entity,
            String,
            [i32; 2],
            [i32; 2],
            bool,
            Vec<String>,
            f32,
            bool,
        )> = Vec::new();
        {
            let entities = self.world.entities();
            let sprites = self.world.read_storage::<SpriteComponent>();
            let tags = self.world.read_storage::<ObjectTag>();
            let group_comps = self.world.read_storage::<GroupComponent>();
            use crate::core::texture_pack as pack;
            for (entity, tag, sprite) in (&entities, &tags, &sprites).join() {
                let name = tag.name.as_str();
                let Some(obj) = crate::data::catalog()
                    .objects
                    .iter()
                    .find(|o| o.name == name)
                else {
                    continue;
                };
                let count = pack::object_atlas(name, obj.texture_count);
                let frame_base = pack::object_frame(name, obj.texture_frame);
                let frames = pack::object_frames(name, obj.frame_paths);
                // box/rack меняют путь по количеству еды — оставляем их
                // логические пути, файловый оверлей пака применяется при загрузке.
                let stateful = name == "box" || name == "rack";
                let animated = if stateful {
                    false
                } else {
                    pack::object_animated(name, obj.animated)
                };
                let mut path = sprite.texture_path.to_string();
                let mut frame = frame_base;
                if let Some(info) = group_comps
                    .get(entity)
                    .and_then(|g| groups.get(&g.group_id))
                {
                    if info.width * info.height > 1 && count == [1, 1] {
                        if !stateful {
                            let base = pack::object_texture(name, obj.path);
                            path = format!("{}@{}x{}", base, info.width, info.height);
                        }
                        frame = [0, 0];
                    } else {
                        let index =
                            info.entities.iter().position(|&e| e == entity).unwrap_or(0) as i32;
                        let i = index / info.height.max(1);
                        let j = index % info.height.max(1);
                        frame = [
                            (frame_base[0] + i) % count[0],
                            (frame_base[1] + j) % count[1],
                        ];
                        if !stateful {
                            path = pack::object_texture(name, obj.path);
                        }
                    }
                } else if !stateful {
                    path = pack::object_texture(name, obj.path);
                }
                changes.push((
                    entity,
                    path,
                    frame,
                    count,
                    animated,
                    frames,
                    pack::object_fps(name),
                    pack::object_loop(name),
                ));
            }
        }
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (entity, path, frame, count, animated, frames, fps, looping) in changes {
            if let Some(sprite) = sprites.get_mut(entity) {
                sprite.texture_path = Arc::from(path.as_str());
                sprite.texture_frame = frame;
                sprite.texture_count = count;
                sprite.animated = animated;
                sprite.frame_paths = frames;
                sprite.current_frame = 0;
                sprite.anim_fps = fps;
                sprite.anim_loop = looping;
                sprite.anim_timer = 0.0;
            }
        }
    }

    // Полная очистка мира между уровнями: удаляет все сущности,
    // сбрасывает кэши, счётчики, карту и ресурсы до начального состояния.
    pub fn clear_world(&mut self) {
        use specs::Join;
        let delete_entities: Vec<specs::Entity> = {
            let entities = self.world.entities();
            let transforms = self.world.read_storage::<Transform>();
            let sprites = self.world.read_storage::<SpriteComponent>();
            (&entities, &transforms, &sprites)
                .join()
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
        self.fence_paths.clear();
        self.cursor_preview.clear();
        self.pending_food_adds.clear();
        self.pending_placement_fx.clear();
        self.next_group_id = 1;
        self.clear_count += 1;
        self.map_grid.clear();
        self.map_entities.clear();
        self.original_tokens.clear();
        self.map_background = None;
        self.wall_positions.clear();
        self.floor_positions.clear();
        self.outdoor_positions.clear();
        self.flower_positions.clear();
        self.world
            .write_resource::<crate::GroupInfoResource>()
            .groups
            .clear();
        self.world.write_resource::<BasementPlaced>().0 = false;
        // Дверь магазина возвращается в закрытое состояние вместе с миром.
        *self.world.write_resource::<DoorProgress>() = DoorProgress::new();
    }
}
