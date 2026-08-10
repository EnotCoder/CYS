use std::collections::{HashMap, HashSet};
use specs::{WorldExt, Join};
use winit::keyboard::KeyCode;
use serde::{Serialize, Deserialize};
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::constants::*;
use crate::inventory::Inventory;
use crate::map::pathfinding::Node;
use crate::ecs::components::{FoodStorage, ObjectTag, TotalFood, BusyCassas, Money};
use crate::scene::game::day_night::DayNightCycle;
use crate::scene::game::hud::GameHud;
use crate::scene::game::shoppers::ShopperManager;

mod day_night;
mod hud;
mod shoppers;

// ========================================================================
//  GameScene — основная игровая сцена
// ========================================================================
//  Содержит весь игровой цикл: загрузку уровней (магазин и подвал),
//  режимы строительства (build/del/interact), управление камерой, слоты
//  хотбара и инвентарь, сохранение/загрузку состояния, покупку и продажу.
//  На шкале кадра: сначала обрабатывается ввод, затем камера, потом
//  обновление компонентов (еда, заборы, HUD) и покупатели с днём/ночью.

/// Мгновенный снимок объекта группы для сохранения состояния уровня
#[derive(Clone)]
struct SavedObject {
    slot_name: String,
    x: i32,
    y: i32,
    food_count: i32,
    max_food: i32,
    is_carpet: bool,
}

/// Полное состояние одного уровня: карта, токены и размещённые объекты
struct LevelState {
    map_grid: Vec<Vec<String>>,
    original_tokens: HashMap<(i32, i32), String>,
    objects: Vec<SavedObject>,
}

pub struct GameScene {
    loaded: bool,
    loading: bool,
    // Хотбар: список слотов-предметов, активный слот и режим игры (build/del/interact)
    slots: Vec<crate::data::Slot>,
    act_slot: i32,
    mode: i32,
    map_size: f32,
    zoom_step: f32,
    // UI-сущности: курсор, иконка режима, рамка выбора слота, блокировка кассы
    cursor_entity: Option<specs::Entity>,
    icon_mode: Option<specs::Entity>,
    icons_slot_cursor: Option<specs::Entity>,
    slot_entities: Vec<specs::Entity>,
    inventory: Inventory,
    // Проходимые клетки для поиска пути покупателей
    npc_walkable: HashSet<Node>,
    last_frame: std::time::Instant,
    anim_timer: f64,
    // Позиция и зум камеры
    camera_offset_x: f32,
    camera_offset_y: f32,
    // Всплывающая надпись "Minecraft" после установки магазина
    ilm_entity: Option<specs::Entity>,
    ilm_timer: f64,
    ilm_cooldown: f64,
    // Таймер регенерации еды в ящиках
    food_timer: f64,
    // Магазин открыт для покупателей или закрыт (иконка ACTIVE)
    active: bool,
    active_entity: Option<specs::Entity>,
    settings: crate::ui::settings::Settings,
    inv_entity: Option<specs::Entity>,
    // Уровни: 0 — магазин, -1 — подвал; кэш состояний при хождении между ними
    current_level: i32,
    level_states: HashMap<i32, LevelState>,
    config: crate::script::config::BalanceConfig,
    npc_script: Option<crate::script::npc::NpcScript>,
    hud: GameHud,
    shoppers: ShopperManager,
    day_night: DayNightCycle,
}

impl GameScene {
    pub fn new() -> Self {
        let config = crate::script::config::BalanceConfig::load();
        GameScene {
            loaded: false,
            loading: false,
            slots: Vec::new(),
            act_slot: 0,
            mode: 0,
            map_size: 0.8,
            zoom_step: 0.1,
            cursor_entity: None,
            icon_mode: None,
            icons_slot_cursor: None,
            slot_entities: Vec::new(),
            inventory: Inventory::new(),
            npc_walkable: HashSet::new(),
            last_frame: std::time::Instant::now(),
            anim_timer: 0.0,
            camera_offset_x: 0.0,
            camera_offset_y: 0.0,
            ilm_entity: None,
            ilm_timer: 0.0,
            ilm_cooldown: 0.0,
            food_timer: 0.0,
            active: true,
            active_entity: None,
            settings: crate::ui::settings::Settings::new(),
            inv_entity: None,
            current_level: 0,
            level_states: HashMap::new(),
            config,
            npc_script: Some(crate::script::npc::NpcScript::new()),
            hud: GameHud::new(),
            shoppers: ShopperManager::new(),
            day_night: DayNightCycle::new(),
        }
    }

    fn show_loading(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.hud.show_loading(ecs, text_renderer, device, queue);
    }

    fn hide_loading(&mut self, ecs: &mut crate::EcsAdapter) {
        self.hud.hide_loading(ecs);
    }

    /// Собирает контент сцены: UI и, для магазина (уровень 0), загрузку карты
    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.setup_ui(ecs, text_renderer, device, queue);
        if self.current_level == 0 {
            crate::map::load_map_to_ecs(ecs);
            self.npc_walkable = crate::map::load_walkable_cells();
        }
    }

    /// Строит постоянный UI: слоты хотбара, иконки режимов, курсор, инфо-панель
    fn setup_ui(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.slots = crate::data::get_slot_vec();

        text_renderer.add_text(ecs, device, queue, "Alpha", FONT_SIZE_ALPHA, -5.5, 4.0, 1.0, 4.0, WHITE);

        // Иконки режима игры, активного состояния и кнопки инвентаря
        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[0]);
        let active_entity = ecs.add_ui(ACTIVE_X, SLOT_BAR_Y, TEX_ACTIVE);
        self.active_entity = Some(active_entity);
        let inv_entity = ecs.add_ui(INV_BTN_X, SLOT_BAR_Y, TEX_INV_BUTTON);
        self.inv_entity = Some(inv_entity);

        // Иконки всех слотов хотбара
        for (i, slot) in self.slots.iter().enumerate() {
            let icon_path = crate::util::slot_icon_path(slot.obj.name);
            let ent = ecs.add_ui(
                SLOT_BAR_X + i as f32, SLOT_BAR_Y,
                &icon_path,
            );
            self.slot_entities.push(ent);
        }

        // Рамка выбора активного слота и игровой курсор
        let icons_slot_cursor = ecs.add_ui(SLOT_BAR_X, SLOT_BAR_Y, SLOT_CURSOR_TEX);
        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, CURSOR_TEX[0]));

        self.hud.create_info_panel(ecs, device, queue);
        self.npc_walkable = crate::map::load_walkable_cells();
    }

    /// Двигает кадры анимации для всех анимированных спрайтов раз в секунду
    fn update_animations(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        self.anim_timer += dt;
        if self.anim_timer >= 1.0 {
            self.anim_timer -= 1.0;
            let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
            for sprite in (&mut sprites).join() {
                if sprite.animated {
                    let n = sprite.frame_paths.len() as i32;
                    sprite.current_frame = (sprite.current_frame + 1) % n;
                    if let Some(path) = sprite.frame_paths.get(sprite.current_frame as usize) {
                        sprite.texture_path = std::sync::Arc::from(path.as_str());
                    }
                }
            }
        }
    }

    /// Обработка кликов по инвентарю: таб, сетка предметов, слоты хотбара
    fn handle_inventory_input(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) {
        // Клавиша E — открыть/закрыть инвентарь
        if input.key_pressed(winit::keyboard::KeyCode::KeyE) {
            if self.inventory.open {
                self.inventory.exit(ecs);
            } else {
                self.inventory.enter(ecs);
            }
        }

        let click = input.mouse_pressed(winit::event::MouseButton::Left);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);

        // --- Клик по табам ---
        if self.inventory.mode {
            let tcol = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            if (wy - INV_TAB_Y).abs() < TILE_HALF && tcol >= 0 && tcol < TAB_TEX.len() as i32 {
                if tcol != self.inventory.tab {
                    self.inventory.switch_tab(tcol, ecs);
                }
                return;
            }
        }

        // --- Клик по сетке инвентаря ---
        if self.inventory.mode {
            let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
            if self.inventory.handle_grid_click(col, row) {
                // Предмет из инвентаря переносится в выбранный слот хотбара
                self.inventory.transfer_to_slot(ecs, self.act_slot as usize, &mut self.slots, &self.slot_entities);
            }
            return;
        }

        // --- Клик по слотам хотбара ---
        let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
        if (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < self.slots.len() as i32 {
            let target = col;
            if target != self.act_slot {
                // Деактивируем старый слот, активируем новый и двигаем рамку выбора
                if let Some(cursor) = self.icons_slot_cursor {
                    let old = self.act_slot as usize;
                    if old < self.slots.len() {
                        self.slots[old].active = false;
                    }
                    self.act_slot = target;
                    self.slots[target as usize].active = true;
                    ecs.update_transform_position(cursor, SLOT_BAR_X + col as f32, SLOT_BAR_Y);
                }
            }
        }
    }

    /// Сохраняет состояние текущего уровня в память (при переключении на другой)
    fn save_current_level(&mut self, ecs: &mut crate::EcsAdapter) {
        let mut objects = Vec::new();
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
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

    /// Переключает игроков между магазином (0) и подвалом (-1).
    /// Сохраняет текущий уровень, очищает мир и строит новый из кэша
    /// level_states либо из файлов карты.
    fn load_level(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, level: i32, skip_save: bool) {
        if !skip_save {
            self.save_current_level(ecs);
        }

        // Закрываем открытый инвентарь и чистим мир под новый уровень
        if self.inventory.open {
            self.inventory.exit(ecs);
        }

        ecs.clear_world();
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
                let (tex, frame, count) = crate::map::token_to_texture(&token);
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
                let slot = crate::data::make_slot(&obj.slot_name);
                let is_carpet = crate::data::is_carpet_name(&obj.slot_name);
                let _is_outdoor = crate::data::is_outdoor_name(&obj.slot_name);
                let _is_flower = crate::data::is_flower_name(&obj.slot_name);
                let _is_wall_decor = crate::data::is_wall_decor_name(&obj.slot_name);
                let new_group_id = ecs.add_group_object(
                    obj.x, obj.y,
                    slot.obj.width, slot.obj.height,
                    slot.obj.path,
                    slot.obj.texture_frame,
                    slot.obj.texture_count,
                    is_carpet,
                    slot.obj.animated,
                    slot.obj.frame_paths,
                );
                let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
                if let Some(info) = groups.groups.get(&new_group_id) {
                    if let Some(&entity) = info.entities.first() {
                        let tag = ObjectTag { name: obj.slot_name.clone() };
                        ecs.world.write_storage::<crate::ObjectTag>().insert(entity, tag).ok();
                        // Восстанавливаем специфичные компоненты по имени объекта
                        if obj.slot_name == "basement" {
                            ecs.world.write_resource::<crate::ecs::components::BasementPlaced>().0 = true;
                        } else if obj.slot_name == "rack" || obj.slot_name == "box" || obj.slot_name == "candies" {
                            ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                                food_count: obj.food_count,
                                max_food: obj.max_food,
                            }).ok();
                        }
                        if obj.slot_name == "fence" || obj.slot_name == "street_fence" {
                            ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent { name: obj.slot_name.clone() }).ok();
                        }
                    }
                }
            }
            ecs.update_fence_textures();
        } else {
            // Уровень ещё не открывался — строим с нуля из файла карты
            if level == -1 {
                crate::map::load_basement_to_ecs(ecs);
                self.place_basement_exit(ecs);
            } else {
                crate::map::load_map_to_ecs(ecs);
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
    fn rebuild_ui(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.slot_entities.clear();
        for (i, slot) in self.slots.iter().enumerate() {
            let icon_path = crate::util::slot_icon_path(slot.obj.name);
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

        text_renderer.add_text(ecs, device, queue, "Alpha", FONT_SIZE_ALPHA, -5.5, 4.0, 1.0, 4.0, WHITE);
        self.npc_walkable = crate::map::load_walkable_cells();
    }

    /// Сохраняет игру в файл save.json (Ctrl+S)
    fn save_to_disk(&mut self, ecs: &mut crate::EcsAdapter) {
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
        let basement_placed = ecs.world.read_resource::<crate::ecs::components::BasementPlaced>().0;
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
            let _ = std::fs::write("save.json", json);
        }
    }

    /// Загружает игру из файла save.json (Ctrl+L)
    fn load_from_disk(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let content = match std::fs::read_to_string("save.json") {
            Ok(c) => c,
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

        // Восстанавливаем глобальные ресурсы мира и настройки игрока
        ecs.clear_world();
        ecs.world.write_resource::<BusyCassas>().0 = data.busy_cassas.into_iter().collect();
        ecs.world.write_resource::<Money>().0 = data.money;
        ecs.world.write_resource::<TotalFood>().0 = data.total_food;
        ecs.world.write_resource::<crate::ecs::components::BasementPlaced>().0 = data.basement_placed;

        self.slots = data.slots.iter().map(|n| crate::data::make_slot(n)).collect();
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
    fn place_basement_exit(&mut self, ecs: &mut crate::EcsAdapter) {
        let gid = ecs.add_group_object(
            -6, 3, 1, 2,
            "tex/decor/regular/basement.png",
            [0, 1], [1, 2],
            false, false, &[],
        );
        let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
        if let Some(info) = groups.groups.get(&gid) {
            if let Some(&entity) = info.entities.first() {
                ecs.world.write_storage::<crate::ObjectTag>().insert(entity, ObjectTag { name: "basement".to_string() }).ok();
            }
        }
        ecs.world.write_resource::<crate::ecs::components::BasementPlaced>().0 = true;
    }
}

impl Scene for GameScene {
    /// Вход в игру: полный сброс всех систем и ресурсов, запуск музыки.
    /// Построение карты откладывается до первого update().
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer) {
        self.loaded = false;
        self.loading = true;
        self.slots = Vec::new();
        self.act_slot = 0;
        self.mode = 0;
        self.map_size = 0.8;
        self.cursor_entity = None;
        self.icon_mode = None;
        self.icons_slot_cursor = None;
        self.slot_entities.clear();
        self.inventory.reset();
        self.npc_walkable.clear();
        // Переносим балансовые настройки в мир ECS для использования другими системами
        *ecs.world.write_resource::<crate::script::config::BalanceConfig>() = self.config.clone();
        self.last_frame = std::time::Instant::now();
        self.anim_timer = 0.0;
        self.camera_offset_x = 0.0;
        self.camera_offset_y = 0.0;
        self.ilm_entity = None;
        self.ilm_timer = 0.0;
        self.ilm_cooldown = 0.0;
        self.food_timer = 0.0;
        self.active = true;
        self.active_entity = None;
        self.inv_entity = None;
        self.hud.reset();
        self.shoppers.clear();
        self.day_night.reset();
        ecs.world.write_resource::<TotalFood>().0 = 0;
        ecs.world.write_resource::<BusyCassas>().0.clear();
        crate::audio::play_music("music");
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        // Отложенная загрузка: показываем "Loading..." на один кадр, затем строим контент
        if !self.loaded {
            if self.loading {
                self.loading = false;
                self.show_loading(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
            self.hide_loading(ecs);
            self.loaded = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        // --- Toggle settings ---
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            if self.settings.open {
                self.settings.close(ecs);
            } else {
                self.settings.open(ecs, text_renderer, device, queue);
            }
        }

        // --- Save / Load ---
        if input.held_control() && input.key_pressed(KeyCode::KeyS) {
            self.save_to_disk(ecs);
        }
        if input.held_control() && input.key_pressed(KeyCode::KeyL) {
            self.load_from_disk(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }

        if self.settings.open {
            // Пока открыты настройки — обрабатываем только их ввод
            self.settings.handle_input(ecs, text_renderer, device, queue, input, window_size);
            if self.settings.vsync_toggled {
                self.settings.vsync_toggled = false;
                let enabled = self.settings.vsync.checked;
                return SceneAction::VsyncToggle(enabled);
            }
            if self.settings.zoom_speed_changed {
                self.settings.zoom_speed_changed = false;
                self.zoom_step = self.settings.zoom_speed.value;
            }
        } else {
            let cursor = self.cursor_entity.unwrap();
            let icon_mode = self.icon_mode.unwrap();
            let icons_slot_cursor = self.icons_slot_cursor.unwrap();

            // Основной игровой ввод: курсор, размещение/удаление/взаимодействие
            let result = crate::input::do_input(
                input, ecs, &mut self.slots, self.act_slot, self.mode, self.map_size, self.zoom_step,
                window_size, cursor, icon_mode, icons_slot_cursor, self.inventory.mode,
                self.camera_offset_x, self.camera_offset_y,
            );
            self.act_slot = result.0;
            self.mode = result.1;
            self.map_size = result.2;
            let show_ilm = result.3;
            let switch_level = result.4;

            // --- Inv button (toggle inventory) ---
            if input.mouse_pressed(winit::event::MouseButton::Left) {
                if let Some((mx, my)) = input.cursor() {
                    let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
                    if (wx - INV_BTN_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        if self.inventory.open {
                            self.inventory.exit(ecs);
                        } else {
                            self.inventory.enter(ecs);
                        }
                    }
                }
            }

            // --- Toggle active/not active ---
            if input.mouse_pressed(winit::event::MouseButton::Left) && !self.inventory.mode {
                if let Some((mx, my)) = input.cursor() {
                    let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
                    // Кнопка "открыт/закрыт": переключает доступность магазина для покупателей
                    if (wx - ACTIVE_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        self.active = !self.active;
                        let tex = if self.active { TEX_ACTIVE } else { TEX_NO_ACTIVE };
                        if let Some(entity) = self.active_entity {
                            ecs.update_sprite_texture(entity, tex);
                        }
                        self.shoppers.set_active(self.active);
                    }
                    // --- Клик по mode ---
                    if (wx - ICON_MODE_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        let cursor = self.cursor_entity.unwrap();
                        let icon_mode = self.icon_mode.unwrap();
                        self.mode = crate::input::interact::cycle_mode(self.mode, ecs, cursor, icon_mode);
                    }
                }
            }

            // --- Switch level ---
            // Клик по спуску/выходу подвала: переключение между магазином и подвалом
            if switch_level == 2 {
                let new_level = if self.current_level == 0 { -1 } else { 0 };
                self.load_level(ecs, text_renderer, device, queue, new_level, false);
                return SceneAction::None;
            }

            // Пасхалка: всплывающая надпись после установки магазина (с cooldown)
            if show_ilm && self.ilm_cooldown <= 0.0 && self.ilm_entity.is_none() {
                let ent = text_renderer.add_text(ecs, device, queue, "Minecraft", 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
                self.ilm_entity = Some(ent);
                self.ilm_timer = 2.0;
                self.ilm_cooldown = 5.0;
            }

            self.handle_inventory_input(ecs, input, window_size);
        }

        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;

        // Прогресс дня/ночи в независимости от режима настроек
        self.day_night.tick(dt);

        // Расчёт видимой области карты для ограничения движения камеры
        let aspect = window_size.0 / window_size.1;
        let vis_w = 2.0 * aspect / (SHADER_SCALE * self.map_size);
        let vis_h = 2.0 / (SHADER_SCALE * self.map_size);
        let cam_min_x = CAMERA_MAP_MIN_X + vis_w / 2.0;
        let cam_max_x = CAMERA_MAP_MAX_X - vis_w / 2.0;
        let cam_min_y = CAMERA_MAP_MIN_Y + vis_h / 2.0;
        let cam_max_y = CAMERA_MAP_MAX_Y - vis_h / 2.0;

        let step = CAMERA_SPEED * (dt as f32);

        // Камера: перемещение зажатой средней кнопкой мыши
        if input.mouse_held(winit::event::MouseButton::Middle) {
            let sensitivity = 0.01;
            let (dx, dy) = input.cursor_diff();
            self.camera_offset_x = (self.camera_offset_x - dx * sensitivity).clamp(cam_min_x, cam_max_x);
            self.camera_offset_y = (self.camera_offset_y + dy * sensitivity).clamp(cam_min_y, cam_max_y);
        }

        // Камера: перемещение стрелками клавиатуры
        if input.key_held(KeyCode::ArrowLeft) {
            self.camera_offset_x = (self.camera_offset_x - step).max(cam_min_x);
        }
        if input.key_held(KeyCode::ArrowRight) {
            self.camera_offset_x = (self.camera_offset_x + step).min(cam_max_x);
        }
        if input.key_held(KeyCode::ArrowDown) {
            self.camera_offset_y = (self.camera_offset_y - step).max(cam_min_y);
        }
        if input.key_held(KeyCode::ArrowUp) {
            self.camera_offset_y = (self.camera_offset_y + step).min(cam_max_y);
        }
        self.camera_offset_x = self.camera_offset_x.clamp(cam_min_x, cam_max_x);
        self.camera_offset_y = self.camera_offset_y.clamp(cam_min_y, cam_max_y);

        // --- Обновление всех объектов по компонентам ---
        // Периодическая регенерация еды в ящиках (box)
        self.food_timer += dt;
        if self.food_timer >= self.config.food_regen_tick {
            self.food_timer -= self.config.food_regen_tick;
            {
                let tags = ecs.world.read_storage::<ObjectTag>();
                let mut foods = ecs.world.write_storage::<FoodStorage>();
                for (tag, storage) in (&tags, &mut foods).join() {
                    if tag.name == "box" && storage.food_count < storage.max_food {
                        storage.food_count += self.config.food_regen_amount;
                    }
                }
            }
            ecs.update_object_textures();
        }
        ecs.update_fence_textures();
        // Определяем объект под курсором для подсказки о запасах еды
        let cursor_pos = self.cursor_entity.map(|e| ecs.get_transform_position(e));
        let hovered_object = cursor_pos.and_then(|(cx, cy)| {
            let gx = cx as i32;
            let gy = cy as i32;
            if let Some(gid) = ecs.find_group_at_position(gx, gy) {
                let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
                if let Some(info) = groups.groups.get(&gid) {
                    if let Some(first) = info.entities.first() {
                        let foods = ecs.world.read_storage::<FoodStorage>();
                        let tags = ecs.world.read_storage::<ObjectTag>();
                        if let Some(f) = foods.get(*first) {
                            let name = tags.get(*first).map(|t| t.name.clone()).unwrap_or("Object".to_string());
                            return Some((f.food_count, f.max_food, name));
                        }
                    }
                }
            }
            None
        });
        self.hud.update_hover(ecs, text_renderer, device, queue, hovered_object);

        // --- Tooltip для ячеек инвентаря ---
        let slot_tooltip = if self.inventory.mode {
            // Ищем предмет под курсором в сетке инвентаря
            input.cursor().and_then(|(mx, my)| {
                let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
                let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
                let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
                if col >= 0 && col < INVENTORY_COLS && row >= 0 && row < INVENTORY_ROWS {
                    let item_idx = crate::util::inventory_index(row, col) as usize;
                    let items = self.inventory.items();
                    if item_idx < items.len() {
                        let name = items[item_idx];
                        Some((name.to_string(), SLOT_BAR_X + col as f32, INVENTORY_BASE_Y + row as f32 - 0.55))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            None
        };
        self.hud.update_slot_tooltip(ecs, text_renderer, device, queue, slot_tooltip);

        // --- Статистика (еда / деньги) ---
        let total_food = ecs.world.read_resource::<TotalFood>().0;
        let money = ecs.world.read_resource::<Money>().0;
        self.hud.update_stats(ecs, text_renderer, device, queue, total_food, money);

        // --- Время на часах ---
        let time_str = self.day_night.time_string(&self.config);
        self.hud.update_time(ecs, text_renderer, device, queue, &time_str, dt);

        // Управление таймером пасхалки "Minecraft"
        if self.ilm_cooldown > 0.0 {
            self.ilm_cooldown -= dt;
        }
        if let Some(ent) = self.ilm_entity {
            self.ilm_timer -= dt;
            if self.ilm_timer <= 0.0 {
                ecs.delete_entity(ent);
                self.ilm_entity = None;
            }
        }

        // --- Shopper NPCs ---
        self.shoppers.tick(ecs, dt, &self.npc_walkable, self.active, &self.config, self.npc_script.as_ref());

        self.update_animations(ecs, dt);

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter, visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        // Отдаём слои рендера с отсечением по видимой области
        ecs.get_sprites_by_layer(visible_bounds)
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }

    fn camera_offset(&self) -> (f32, f32) {
        (self.camera_offset_x, self.camera_offset_y)
    }

    /// Коэффициент затенения для шейдера (ночь затемняет мир)
    fn night_factor(&self) -> f32 {
        self.day_night.factor(&self.config)
    }
}