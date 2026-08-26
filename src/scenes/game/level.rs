// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  level.rs — управление уровнями, сохранение/загрузка и переходы
// ========================================================================
//  Содержит логику переключения между магазином (0) и подвалом (-1),
//  кэширование состояний уровней, ручное сохранение на диск (Ctrl+S)
//  и загрузку с диска (Ctrl+L), а также установку выхода из подвала.
// ========================================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use specs::WorldExt;
use wgpu::{Device, Queue};
use crate::EcsAdapter;
use crate::core::constants::*;
use crate::ecs::components::{BasementPlaced, BusyCassas, FenceComponent, FoodStorage, Money, ObjectTag, TotalFood};
use crate::data::{attach_point_light, is_carpet_name, is_flower_name, is_light_name, is_outdoor_name, is_wall_decor_name, make_slot};
use crate::data::map::{load_basement_to_ecs, load_map_to_ecs, load_walkable_cells, token_to_texture};
use crate::ui::text_renderer::TextRenderer;
use crate::GroupInfoResource;
use super::GameScene;

/// Мгновенный снимок объекта группы для сохранения состояния уровня
#[derive(Clone)]
pub struct SavedObject {
    pub slot_name: String,
    pub x: i32,
    pub y: i32,
    pub food_count: i32,
    pub max_food: i32,
    pub is_carpet: bool,
}

/// Полное состояние одного уровня: карта, токены и размещённые объекты
pub struct LevelState {
    pub map_grid: Vec<Vec<String>>,
    pub original_tokens: HashMap<(i32, i32), String>,
    pub objects: Vec<SavedObject>,
}

impl GameScene {
    /// Сохраняет состояние текущего уровня в память (при переключении на другой)
    pub fn save_current_level(&mut self, ecs: &mut EcsAdapter) {
        let mut objects = Vec::new();
        let groups = ecs.world.read_resource::<GroupInfoResource>();
        let tags = ecs.world.read_storage::<ObjectTag>();
        let foods = ecs.world.read_storage::<FoodStorage>();
        // Перебираем все группы объектов и собираем их параметры
        for (_, group) in &groups.groups {
            let name = group.entities.first()
                .and_then(|e| tags.get(*e))
                .map(|t| t.name.as_str())
                .unwrap_or("");
            let food_storage = group.entities.first()
                .and_then(|e| foods.get(*e));
            objects.push(SavedObject {
                slot_name: name.to_string(),
                x: group.pos_x,
                y: group.pos_y,
                food_count: food_storage.map_or(0, |f| f.food_count),
                max_food: food_storage.map_or(0, |f| f.max_food),
                is_carpet: group.is_carpet,
            });
        }
        self.level_states.insert(self.current_level, LevelState {
            map_grid: ecs.map_grid.clone(),
            original_tokens: ecs.original_tokens.clone(),
            objects,
        });
    }

    /// Переключает игрока между магазином (0) и подвалом (-1).
    /// Сохраняет текущий уровень, очищает мир и строит новый из кэша
    /// level_states либо из файлов карты.
    pub fn load_level(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &Device,
        queue: &Queue,
        level: i32,
        skip_save: bool,
    ) {
        if !skip_save {
            self.save_current_level(ecs);
        }

        // Закрываем открытый инвентарь и чистим мир под новый уровень
        if self.inventory.open {
            self.inventory.exit(ecs);
        }

        ecs.clear_world();
        self.clear_food_fx();
        ecs.world.write_resource::<BusyCassas>().0.clear();

        self.current_level = level;
        ecs.current_level = level;

        // Восстановление сохранённого состояния уровня (возврат с другого)
        if let Some(state) = self.level_states.get(&level) {
            ecs.map_grid = state.map_grid.clone();
            ecs.original_tokens = state.original_tokens.clone();
            // Пересоздаём спрайты земли для каждой сохранённой клетки
            for (pos, _) in ecs.original_tokens.clone() {
                let token = ecs.original_tokens.get(&pos).cloned().unwrap_or_default();
                let (tex, frame, count) = token_to_texture(&token);
                let (wx, wy) = (pos.0 as f32, pos.1 as f32);
                let entity = crate::ecs::factory::create_sprite(&mut ecs.world, wx, wy, Z_MAP, tex, frame, count, 1.0, 1.0);
                ecs.map_entities.insert(pos, entity);
                ecs.map_grid[(-wy + WORLD_OFFSET_Y) as usize][(wx + -WORLD_OFFSET_X) as usize] = token;
            }
            // Восстанавливаем вспомогательные множества (стены, пол, трава и т.д.)
            for (j, row) in ecs.map_grid.iter().enumerate() {
                for (i, token) in row.iter().enumerate() {
                    let x = i as f32 + WORLD_OFFSET_X;
                    let y = -(j as f32) + WORLD_OFFSET_Y;
                    let gx = (x + 0.5).floor() as i32;
                    let gy = (y + 0.5).floor() as i32;
                    let is_grass = matches!(token.as_str(), "." | "@" | "*" | "m" | "f" | "~" | "l" | "1" | "2" | "3" | "4" | "5" | "6");
                    if token == "=" || token == "-" {
                        ecs.wall_positions.insert((gx, gy));
                    } else if token == "0" {
                        ecs.floor_positions.insert((gx, gy));
                    }
                    if is_grass {
                        ecs.outdoor_positions.insert((gx, gy));
                        ecs.flower_positions.insert((gx, gy));
                    }
                    if matches!(token.as_str(), "/" | "|" | ".") {
                        ecs.floor_placeable_positions.insert((gx, gy));
                    } else if token == "&" {
                        let is_bottom_wall = j > 0 && ecs.map_grid.get(j - 1)
                            .and_then(|r| r.get(i))
                            .map_or(false, |t| t == "0");
                        if !is_bottom_wall {
                            ecs.floor_placeable_positions.insert((gx, gy));
                        }
                    }
                }
            }
            // Восстанавливаем размещённые объекты: спрайты, теги, хранилища еды
            for obj in &state.objects {
                let slot = make_slot(&obj.slot_name);
                let is_carpet = is_carpet_name(&obj.slot_name);
                let is_light = is_light_name(&obj.slot_name);
                let _is_outdoor = is_outdoor_name(&obj.slot_name);
                let _is_flower = is_flower_name(&obj.slot_name);
                let _is_wall_decor = is_wall_decor_name(&obj.slot_name);
                let new_group_id = ecs.add_group_object(
                    obj.x, obj.y,
                    slot.obj.width, slot.obj.height,
                    slot.obj.path,
                    slot.obj.texture_frame,
                    slot.obj.texture_count,
                    is_carpet,
                    is_light,
                    slot.obj.animated,
                    slot.obj.frame_paths,
                );
                let groups = ecs.world.read_resource::<GroupInfoResource>();
                if let Some(info) = groups.groups.get(&new_group_id) {
                    if let Some(&entity) = info.entities.first() {
                        let tag = ObjectTag { name: obj.slot_name.clone() };
                        ecs.world.write_storage::<ObjectTag>().insert(entity, tag).ok();
                        // Восстанавливаем специфичные компоненты по имени объекта
                        if obj.slot_name == "basement" {
                            ecs.world.write_resource::<BasementPlaced>().0 = true;
                        } else if obj.slot_name == "rack" || obj.slot_name == "box" || obj.slot_name == "candies" {
                            ecs.world.write_storage::<FoodStorage>().insert(entity, FoodStorage {
                                food_count: obj.food_count,
                                max_food: obj.max_food,
                            }).ok();
                        }
                        if obj.slot_name == "fence" || obj.slot_name == "street_fence" {
                            ecs.world.write_storage::<FenceComponent>().insert(entity, FenceComponent { name: obj.slot_name.clone() }).ok();
                        }
                        // Источники света (лампы, автоматы, мороженое, конфеты)
                        // восстанавливаются по имени — иначе после загрузки свет пропадает.
                        attach_point_light(ecs, entity, &obj.slot_name);
                    }
                }
            }
            ecs.update_fence_textures();
        } else {
            // Уровень ещё не открывался — строим с нуля из файла карты
            if level == -1 {
                load_basement_to_ecs(ecs);
                self.place_basement_exit(ecs);
            } else {
                load_map_to_ecs(ecs);
            }
        }

        // Сбрасываем камеру и пересоздаём UI для нового уровня
        self.camera_offset_x = 0.0;
        self.camera_offset_y = 0.0;
        self.map_size = 0.8;

        self.hud.reset();
        self.rebuild_ui(ecs, text_renderer, device, queue);
    }

    /// Пересоздаёт UI-сущности сцены после смены уровня (т.к. мир очищен)
    pub fn rebuild_ui(&mut self, ecs: &mut EcsAdapter, _text_renderer: &mut TextRenderer, device: &Device, queue: &Queue) {
        self.slot_entities.clear();
        for (i, slot) in self.slots.iter().enumerate() {
            let icon_path = crate::core::util::slot_icon_path(slot.obj.name);
            let ent = ecs.add_ui(SLOT_BAR_X + i as f32, SLOT_BAR_Y, &icon_path);
            self.slot_entities.push(ent);
        }
        let cursor_x = SLOT_BAR_X + self.act_slot as f32;
        let icons_slot_cursor = ecs.add_ui(cursor_x, SLOT_BAR_Y, SLOT_CURSOR_TEX);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[self.mode as usize]);
        self.icon_mode = Some(icon_mode);
        let active_entity = ecs.add_ui(ACTIVE_X, SLOT_BAR_Y, TEX_ACTIVE);
        self.active_entity = Some(active_entity);
        let inv_entity = ecs.add_ui(INV_BTN_X, SLOT_BAR_Y, TEX_INV_BUTTON);
        self.inv_entity = Some(inv_entity);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, CURSOR_TEX[self.mode as usize]));

        self.hud.create_info_panel(ecs, device, queue);

        self.npc_walkable = load_walkable_cells();
    }

    /// Сохраняет игру в файл save.json (Ctrl+S)
    pub fn save_to_disk(&mut self, ecs: &mut EcsAdapter) {
        self.save_current_level(ecs);
        #[derive(Serialize)]
        struct ObjSave {
            slot_name: String, x: i32, y: i32,
            food_count: i32, max_food: i32, is_carpet: bool,
        }
        #[derive(Serialize)]
        struct LevelSave {
            map_grid: Vec<Vec<String>>,
            original_tokens: Vec<(i32, i32, String)>,
            objects: Vec<ObjSave>,
        }
        #[derive(Serialize)]
        struct Data {
            levels: HashMap<i32, LevelSave>,
            current_level: i32,
            money: i32, total_food: i32,
            slots: Vec<String>, act_slot: i32, mode: i32,
            camera_offset_x: f32, camera_offset_y: f32, map_size: f32,
            active: bool, basement_placed: bool,
            busy_cassas: Vec<(i32, i32)>,
        }
        let mut levels = HashMap::new();
        // Сериализуем состояния всех уровней в транспортные структуры
        for (&lvl, ls) in &self.level_states {
            let objects: Vec<ObjSave> = ls.objects.iter().map(|o| ObjSave {
                slot_name: o.slot_name.clone(),
                x: o.x, y: o.y,
                food_count: o.food_count, max_food: o.max_food, is_carpet: o.is_carpet,
            }).collect();
            let original_tokens: Vec<(i32, i32, String)> = ls.original_tokens.iter()
                .map(|((x, y), t)| (*x, *y, t.clone()))
                .collect();
            levels.insert(lvl, LevelSave {
                map_grid: ls.map_grid.clone(),
                original_tokens,
                objects,
            });
        }
        // Собираем глобальное состояние мира и UI
        let basement_placed = ecs.world.read_resource::<BasementPlaced>().0;
        let busy_cassas: Vec<(i32, i32)> = ecs.world.read_resource::<BusyCassas>().0.iter().copied().collect();
        let money = ecs.world.read_resource::<Money>().0;
        let total_food = ecs.world.read_resource::<TotalFood>().0;
        let slots: Vec<String> = self.slots.iter().map(|s| s.obj.name.to_string()).collect();
        let data = Data {
            levels,
            current_level: self.current_level,
            money, total_food,
            slots, act_slot: self.act_slot, mode: self.mode,
            camera_offset_x: self.camera_offset_x,
            camera_offset_y: self.camera_offset_y,
            map_size: self.map_size,
            active: self.active,
            basement_placed,
            busy_cassas,
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            if crate::core::asset::save_data("save.json", json.as_bytes()).is_ok() {
                crate::audio::play("save");
            }
        }
    }

    /// Загружает игру из файла save.json (Ctrl+L)
    pub fn load_from_disk(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &Device,
        queue: &Queue,
    ) {
        let content = match crate::core::asset::load_data("save.json") {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(c) => c,
                Err(_) => return,
            },
            Err(_) => return,
        };
        #[derive(Deserialize)]
        struct ObjSave {
            slot_name: String, x: i32, y: i32,
            food_count: i32, max_food: i32, is_carpet: bool,
        }
        #[derive(Deserialize)]
        struct LevelSave {
            map_grid: Vec<Vec<String>>,
            original_tokens: Vec<(i32, i32, String)>,
            objects: Vec<ObjSave>,
        }
        #[derive(Deserialize)]
        struct Data {
            levels: HashMap<i32, LevelSave>,
            current_level: i32,
            money: i32, total_food: i32,
            slots: Vec<String>, act_slot: i32, mode: i32,
            camera_offset_x: f32, camera_offset_y: f32, map_size: f32,
            active: bool, basement_placed: bool,
            busy_cassas: Vec<(i32, i32)>,
        }
        let data: Data = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(_) => return,
        };
        crate::audio::play("save");

        // Восстанавливаем глобальные ресурсы мира и настройки игрока
        ecs.clear_world();
        ecs.world.write_resource::<BusyCassas>().0 = data.busy_cassas.into_iter().collect();
        ecs.world.write_resource::<Money>().0 = data.money;
        ecs.world.write_resource::<TotalFood>().0 = data.total_food;
        ecs.world.write_resource::<BasementPlaced>().0 = data.basement_placed;

        self.slots = data.slots.iter().map(|n| make_slot(n)).collect();
        self.act_slot = data.act_slot;
        self.mode = data.mode;
        self.camera_offset_x = data.camera_offset_x;
        self.camera_offset_y = data.camera_offset_y;
        self.map_size = data.map_size;
        self.active = data.active;
        self.current_level = data.current_level;
        ecs.current_level = data.current_level;

        // Преобразуем декодированные состояния обратно во внутренний формат
        self.level_states.clear();
        for (lvl, ls) in &data.levels {
            let mut original_tokens = HashMap::new();
            for (x, y, t) in &ls.original_tokens {
                original_tokens.insert((*x, *y), t.clone());
            }
            let objects: Vec<SavedObject> = ls.objects.iter().map(|o| SavedObject {
                slot_name: o.slot_name.clone(),
                x: o.x, y: o.y,
                food_count: o.food_count, max_food: o.max_food, is_carpet: o.is_carpet,
            }).collect();
            self.level_states.insert(*lvl, LevelState {
                map_grid: ls.map_grid.clone(),
                original_tokens,
                objects,
            });
        }

        // Строим сохранённый уровень (объекты уже в level_states)
        self.load_level(ecs, text_renderer, device, queue, self.current_level, true);
    }

    /// Отмечает, что подвал установлен у игрока, и рисует его выход в подвал
    pub fn place_basement_exit(&mut self, ecs: &mut EcsAdapter) {
        let gid = ecs.add_group_object(
            -6, 3, 1, 2,
            "tex/decor/regular/basement.png",
            [0, 1], [1, 2],
            false, false, false, &[],
        );
        let groups = ecs.world.read_resource::<GroupInfoResource>();
        if let Some(info) = groups.groups.get(&gid) {
            if let Some(&entity) = info.entities.first() {
                ecs.world.write_storage::<ObjectTag>().insert(entity, ObjectTag { name: "basement".to_string() }).ok();
            }
        }
        ecs.world.write_resource::<BasementPlaced>().0 = true;
    }
}
