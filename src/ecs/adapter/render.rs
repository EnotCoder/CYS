// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Рендер-методы EcsAdapter: get_sprites_by_layer (разбиение всех сущностей
//  по 8 z-слоям с отсечением видимой области), update_object_textures
//  (кадры box/rack по количеству еды), update_fence_textures (заборы по соседям).
// ========================================================================

use specs::{WorldExt, Join};
use std::collections::HashSet;
use std::sync::Arc;
use crate::ecs::components::{Transform, SpriteComponent, Rotation, ObjectTag, FoodStorage, FenceComponent, ShopperTag};
use crate::core::constants::{DOOR_X_MAX, DOOR_X_MIN, DOOR_Y_MAX, DOOR_Y_MIN, DOOR_FADE_RAMP, TILE_HALF, Z_CARPET, Z_CURSOR, Z_DECOR, Z_DOOR, Z_LIGHT, Z_MAP, Z_NPC, Z_WEATHER};
use crate::data::map::{door_cells, door_frame_for_cell, door_texture_for_step};
use crate::GroupComponent;
use super::SpriteRenderData;

/// Насколько покупатель виден у дверного проёма: 1 — полностью, 0 — скрыт.
///
/// Внутри прямоугольника проёма всегда 0, а по краям плавно выходит в 1
/// на расстоянии DOOR_FADE_RAMP. Плавность нужна, чтобы у входа и выхода не
/// было резкого появления: раньше покупатель просто пропускался кадром рендера,
/// и на границе клеток он «щёлкал».
///
/// Затухание двустороннее: покупатель гаснет, заходя с юга (с тротуара), и
/// проявляется, выходя на север (на паркет). Расстояние считается по
/// Чебышёву — до ближайшей грани прямоугольника, а не до угла.
fn doorway_fade(pos: [f32; 3]) -> f32 {
    let half = TILE_HALF;
    let min_x = DOOR_X_MIN as f32 - half;
    let max_x = DOOR_X_MAX as f32 + half;
    let min_y = DOOR_Y_MIN as f32 - half;
    let max_y = DOOR_Y_MAX as f32 + half;
    // Насколько точка снаружи прямоугольника по каждой оси (0 — внутри).
    let dx = (min_x - pos[0]).max(pos[0] - max_x).max(0.0);
    let dy = (min_y - pos[1]).max(pos[1] - max_y).max(0.0);
    let t = (dx.max(dy) / DOOR_FADE_RAMP).clamp(0.0, 1.0);
    // smoothstep: мягкий старт и финиш, без излома на середине.
    t * t * (3.0 - 2.0 * t)
}

impl super::EcsAdapter {
    // Собирает все спрайты мира и раскладывает их по восьми слоям рендера.
    // Возвращает кортеж векторов (карта, ковры, свет, декор, NPC, погода,
    // курсор, UI).
    // `visible_bounds` (l, r, b, t) для карты/декора/NPC включает отсечение
    // по экрану — экономия на запредельных объектах.
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
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
    ) {
        let transforms = self.world.read_storage::<Transform>();
        let sprites = self.world.read_storage::<SpriteComponent>();
        let rotations = self.world.read_storage::<Rotation>();
        let shoppers = self.world.read_storage::<ShopperTag>();

        let margin = 2.0;
        // Векторы заранее резервируются под типичное число объектов на слой.
        let mut map_sprites = Vec::with_capacity(100);
        let mut carpet_sprites = Vec::with_capacity(20);
        let mut light_sprites = Vec::with_capacity(5);
        let mut decor_sprites = Vec::with_capacity(20);
        let mut npc_sprites = Vec::with_capacity(5);
        let mut weather_sprites = Vec::with_capacity(20);
        let mut cursor_sprites = Vec::with_capacity(1);
        let mut ui_sprites = Vec::with_capacity(10);

        // Обход всех сущностей; `maybe()` — опциональные компоненты.
        for (transform, sprite, shopper_opt, rotation_opt) in
            (&transforms, &sprites, shoppers.maybe(), rotations.maybe()).join()
        {
            // Покупатель у дверного проёма гаснет: дверь лежит на Z_DECOR, а
            // её перемычка (9 непрозрачных пикселей сверху) закрывает 38%
            // роста человека, а проём в северной клетке — всего 7 пикселей,
            // то есть шириной 1.12 клетки против 1.5 у покупателя: косяки
            // срезали бы ему бока, и выглядело бы, будто он идёт сквозь стенку.
            // Затухание плавное с обеих сторон, поэтому на границах проёма
            // нет щелчка. Работает при любом состоянии двери: закрытая
            // непрозрачна целиком, так что результат тот же.
            let doorway = doorway_fade(transform.position);
            if shopper_opt.is_some() && doorway <= 0.0 {
                // Полностью скрыт — зачем отправлять в батчет спрайт с
                // нулевой альфой.
                continue;
            }

            let mut alpha = sprite.alpha;
            if shopper_opt.is_some() {
                alpha *= doorway;
            }
            let data = SpriteRenderData {
                position: transform.position,
                rotation: rotation_opt.map(|r| r.rotation).unwrap_or([0.0; 3]),
                texture_path: Arc::clone(&sprite.texture_path),
                texture_frame: sprite.texture_frame,
                texture_count: sprite.texture_count,
                scale: sprite.scale,
                alpha,
            };

            // Уровневые сущности (не UI/курсор) отсекаются по границам экрана:
            // невидимые объекты не попадают в список отрисовки.
            let z = transform.position[2];
            let should_cull = z == Z_MAP
                || z == Z_CARPET
                || z == Z_LIGHT
                || z == Z_DECOR
                || z == Z_NPC;
            if should_cull {
                if let Some((l, r, b, t)) = visible_bounds {
                    let x = transform.position[0];
                    let y = transform.position[1];
                    if x + 1.0 + margin < l || x - margin > r || y + 1.0 + margin < b || y - margin > t {
                        continue;
                    }
                }
            }
            // Распределение по слоям согласно Z-константам (см. AGENTS.md).
            if z == Z_MAP {
                map_sprites.push(data);
            } else if z == Z_CARPET {
                carpet_sprites.push(data);
            } else if z == Z_LIGHT {
                light_sprites.push(data);
            } else if z == Z_DECOR {
                decor_sprites.push(data);
            } else if z == Z_NPC {
                npc_sprites.push(data);
            } else if z == Z_WEATHER {
                weather_sprites.push(data);
            } else if z == Z_CURSOR {
                cursor_sprites.push(data);
            } else {
                ui_sprites.push(data);
            }
        }

        (map_sprites, carpet_sprites, light_sprites, decor_sprites, npc_sprites, weather_sprites, cursor_sprites, ui_sprites)
    }

    // Обновляет текстуры заполненных объектов (box/rack) по количеству еды.
    // Пороги из BalanceConfig: box меняет кадр на tiers, rack — пусто/полно.
    pub fn update_object_textures(&mut self) {
        let cfg = self.world.read_resource::<crate::scripts::config::BalanceConfig>();
        let (t1, t2) = (cfg.box_tex_threshold_1, cfg.box_tex_threshold_2);
        // Накопляем обновления по группам, чтобы не писать в storage во время чтения.
        let mut updates: Vec<(u32, Arc<str>)> = Vec::new();
        {
            let tags = self.world.read_storage::<ObjectTag>();
            let foods = self.world.read_storage::<FoodStorage>();
            let groups = self.world.read_storage::<GroupComponent>();
            for (tag, food, group) in (&tags, &foods, &groups).join() {
                let tex: Arc<str> = if tag.name == "box" {
                    // Три стадии наполнения коробки: пусто / наполовину / полная.
                    if food.food_count < t1 {
                        Arc::from("assets/tex/decor/regular/box/box_0.png")
                    } else if food.food_count < t2 {
                        Arc::from("assets/tex/decor/regular/box/box_1.png")
                    } else {
                        Arc::from("assets/tex/decor/regular/box/box_2.png")
                    }
                } else if tag.name == "rack" {
                    // Стеллаж: либо пустой, либо заполненный.
                    if food.food_count == 0 {
                        Arc::from("assets/tex/decor/regular/rack/rack_0.png")
                    } else {
                        Arc::from("assets/tex/decor/regular/rack/rack_1.png")
                    }
                } else {
                    continue;
                };
                updates.push((group.group_id, tex));
            }
        }
        // Применяем новый путь текстуры ко всем сущностям каждой группы.
        let group_info = self.world.read_resource::<crate::GroupInfoResource>();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (gid, tex) in &updates {
            if let Some(info) = group_info.groups.get(gid) {
                for &entity in &info.entities {
                    if let Some(sprite) = sprites.get_mut(entity) {
                        sprite.texture_path = Arc::clone(tex);
                    }
                }
            }
        }
    }

    // Выбирает текстуру забора по соседям: имя файла кодирует
    // наличие заборов сверху/снизу/слева/справа (например fence_1_0_1_0.png).
    pub fn update_fence_textures(&mut self) {
        let transforms = self.world.read_storage::<Transform>();
        let fences = self.world.read_storage::<FenceComponent>();
        // Множество всех клеток с заборами для быстрой проверки соседства.
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
            // Разные наборы текстур у уличного и обычного заборов.
            let (dir, fallback_arc) = if fence.name == "street_fence" {
                ("assets/tex/decor/outdoor/street_fence/street_fence", Arc::from("assets/tex/decor/outdoor/street_fence/street_fence_0_0_0_0.png"))
            } else {
                ("assets/tex/decor/regular/fence/fence", Arc::from("assets/tex/decor/regular/fence/fence_0_0_0_0.png"))
            };
            let path = format!("{}_{}_{}_{}_{}.png", dir, up as u8, down as u8, left as u8, right as u8);
            // Проверяем наличие варианта через загрузчик ассетов (работает и на
            // Android, где текстуры упакованы в APK, а не лежат файлами на диске).
            if crate::core::asset::load_bytes(&path).is_ok() {
                sprite.texture_path = Arc::from(path.into_boxed_str());
            } else {
                sprite.texture_path = fallback_arc;
            }
        }
    }

    // Текстура двери магазина по прогрессу анимации. Дверь — четыре тайла
    // карты с токеном "E", у каждого свой кадр в атласе 2x2, поэтому
    // подменяем и путь, и кадр: четыре кадра вместе снова собирают
    // цельный проём. Шаг 0 = закрыта, шаг 3 = открыта.
    pub fn update_door_textures(&mut self, step: i32) {
        let path = door_texture_for_step(step);
        for (x, y) in door_cells() {
            let Some(&entity) = self.map_entities.get(&(x, y)) else { continue };
            self.set_door_cell_frame(entity, door_frame_for_cell(x, y));
            self.update_sprite_texture(entity, path);
        }
    }

    // Кадр атласа 2x2 для одной клетки двери.
    pub fn set_door_cell_frame(&mut self, entity: specs::Entity, frame: [i32; 2]) {
        if let Some(sprite) = self.world.write_storage::<SpriteComponent>().get_mut(entity) {
            sprite.texture_frame = frame;
        }
    }

    // Полная настройка клетки двери: свой кадр атласа 2x2 и слой Z_DOOR
    // (поверху покупателей). Вызывается везде, где создаются тайлы карты —
    // и при загрузке, и при восстановлении уровня из кэша.
    pub fn apply_door_cell(&mut self, entity: specs::Entity, x: i32, y: i32) {
        self.set_door_cell_frame(entity, door_frame_for_cell(x, y));
        self.update_transform_z(entity, Z_DOOR);
    }
}
