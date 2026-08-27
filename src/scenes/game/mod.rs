// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  GameScene — главная игровая сцена: цикл (ввод, камера, объекты, HUD,
//  покупатели, день/ночь), режимы build/del/interact, экономика (аренда,
//  банкротство), FoodPulse для «попа еды», настройки и уровни.
// ========================================================================

use std::collections::{HashMap, HashSet};
use specs::{WorldExt, Join};
use winit::keyboard::KeyCode;
use crate::scenes::scene_trait::{Scene, SceneAction};
use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::ui::inventory::Inventory;
use crate::data::map::pathfinding::Node;
use crate::ecs::components::{FoodStorage, ObjectTag, TotalFood, BusyCassas, Money, PlacementError, ShopOwned, ShopDenied};
use crate::scenes::game::day_night::DayNightCycle;
use crate::scenes::game::hud::GameHud;
use crate::scenes::game::shoppers::ShopperManager;

mod camera;
mod day_night;
mod hud;
mod inventory_input;
mod level;
mod shoppers;

pub use level::LevelState;

// Пульс объекта («поп») при добавлении еды: масштаб растёт и возвращается.
struct FoodPulse {
    group_id: u32,
    timer: f64,
    lifetime: f64,
}

// ========================================================================
//  GameScene — основная игровая сцена
// ========================================================================
//  Координирует игровой цикл: загрузку уровней (level), режимы строительства
//  (build/del/interact), управление камерой (camera), слоты хотбара и
//  инвентарь (inventory_input), сохранение/загрузку состояния, покупку и продажу.
//  На шкале кадра: сначала ввод, затем камера, потом обновление компонентов
//  (еда, заборы, HUD) и покупатели с днём/ночью.

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
    // Всплывающая красная подсказка «не хватает денег» при попытке поставить объект
    no_money_entity: Option<specs::Entity>,
    no_money_timer: f64,
    // Таймер регенерации еды в ящиках
    food_timer: f64,
    // Магазин открыт для покупателей или закрыт (иконка ACTIVE)
    active: bool,
    active_entity: Option<specs::Entity>,
    settings: crate::ui::settings::Settings,
    shop: crate::ui::shop::Shop,
    shop_entity: Option<specs::Entity>,
    inv_entity: Option<specs::Entity>,
    // Уровни: 0 — магазин, -1 — подвал; кэш состояний при хождении между ними
    current_level: i32,
    level_states: HashMap<i32, LevelState>,
    config: crate::scripts::config::BalanceConfig,
    npc_script: Option<crate::scripts::npc::NpcScript>,
    hud: GameHud,
    shoppers: ShopperManager,
    day_night: DayNightCycle,
    // Пульсы объектов при появлении еды («поп»)
    food_pulses: Vec<FoodPulse>,
    // Аренда магазина: таймер с момента последнего списания
    rent_timer: f64,
    // Банкротство: магазин не может зарабатывать и на восстановление не хватает денег
    bankrupt: bool,
    // Система миров: id и имя текущего загруженного мира (None до первого входа)
    world_id: Option<u32>,
    world_name: String,
    // Сущность кнопки настроек (иконка gear) в углу экрана
    settings_entity: Option<specs::Entity>,
    bankrupt_bg: Option<specs::Entity>,
    bankrupt_title: Option<specs::Entity>,
    bankrupt_hint: Option<specs::Entity>,
    bankrupt_button: Option<specs::Entity>,
    bankrupt_button_label: Option<specs::Entity>,
    // Пульс рамки выбранного слота хотбара и иконки режима при смене
    slot_pulse: f64,
    mode_pulse: f64,
    prev_act_slot: i32,
    prev_mode: i32,
    // Масштаб UI для текущего соотношения сторон (адаптация под экран)
    ui_scale: f32,
}

impl GameScene {
    pub fn new() -> Self {
        let config = crate::scripts::config::BalanceConfig::load();
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
            no_money_entity: None,
            no_money_timer: 0.0,
            food_timer: 0.0,
            active: true,
            active_entity: None,
            settings: crate::ui::settings::Settings::new(),
            shop: crate::ui::shop::Shop::new(),
            shop_entity: None,
            inv_entity: None,
            current_level: 0,
            level_states: HashMap::new(),
            config,
            npc_script: Some(crate::scripts::npc::NpcScript::new()),
            hud: GameHud::new(),
            shoppers: ShopperManager::new(),
            day_night: DayNightCycle::new(),
            food_pulses: Vec::new(),
            rent_timer: 0.0,
            bankrupt: false,
            world_id: None,
            world_name: String::new(),
            settings_entity: None,
            bankrupt_bg: None,
            bankrupt_title: None,
            bankrupt_hint: None,
            bankrupt_button: None,
            bankrupt_button_label: None,
            slot_pulse: 0.0,
            mode_pulse: 0.0,
            prev_act_slot: 0,
            prev_mode: 0,
            ui_scale: 1.0,
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
            crate::data::map::load_map_to_ecs(ecs);
            self.npc_walkable = crate::data::map::load_walkable_cells();
        }
    }

    /// Строит постоянный UI: слоты хотбара, иконки режимов, курсор, инфо-панель
    fn setup_ui(&mut self, ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.slots = crate::data::get_slot_vec();

        // Иконки режима игры, активного состояния и кнопки инвентаря
        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[0]);
        let active_entity = ecs.add_ui(ACTIVE_X, SLOT_BAR_Y, TEX_ACTIVE);
        self.active_entity = Some(active_entity);
        let inv_entity = ecs.add_ui(INV_BTN_X, SLOT_BAR_Y, TEX_INV_BUTTON);
        self.inv_entity = Some(inv_entity);

        // Кнопка магазина — правее кнопки инвентаря
        let shop_entity = ecs.add_ui(SHOP_BTN_X, SLOT_BAR_Y, TEX_SHOP);
        self.shop_entity = Some(shop_entity);

        // Кнопка настроек (иконка gear) в верхнем левом углу
        let settings_entity = ecs.add_ui(SETTINGS_BTN_X, SETTINGS_BTN_Y, TEX_SETTINGS);
        self.settings_entity = Some(settings_entity);

        // Иконки всех слотов хотбара (сдвинуты на 1: кнопка настроек перед 1-м слотом)
        for (i, slot) in self.slots.iter().enumerate() {
            let icon_path = crate::core::util::slot_icon_path(slot.obj.name);
            let ent = ecs.add_ui(
                HOTBAR_X + i as f32, SLOT_BAR_Y,
                &icon_path,
            );
            self.slot_entities.push(ent);
        }

        // Рамка выбора активного слота и игровой курсор
        let icons_slot_cursor = ecs.add_ui(HOTBAR_X, SLOT_BAR_Y, SLOT_CURSOR_TEX);
        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, CURSOR_TEX[0]));

        self.hud.create_info_panel(ecs, device, queue);
        self.npc_walkable = crate::data::map::load_walkable_cells();
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

        /// Перебирает накопленные сценой/вводом события «еда добавлена» и запускает пульс объекта.
    fn drain_food_fx(&mut self, ecs: &mut crate::EcsAdapter) {
        let pending = std::mem::take(&mut ecs.pending_food_adds);
        for group_id in pending {
            self.food_pulses.push(FoodPulse {
                group_id,
                timer: 0.0,
                lifetime: 0.35,
            });
        }
    }

    /// Каждокадровая анимация «попа» объекта: масштаб растёт и возвращается.
    fn update_food_fx(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        let mut i = 0;
        while i < self.food_pulses.len() {
            self.food_pulses[i].timer += dt;
            let p = (self.food_pulses[i].timer / self.food_pulses[i].lifetime).min(1.0) as f32;
            let group_id = self.food_pulses[i].group_id;
            // Масштаб: 1.0 -> 1.25 -> 1.0 по синусоиде.
            let scale = 1.0 + 0.25 * (std::f32::consts::PI * p).sin();
            ecs.update_group_scale(group_id, scale);
            if p >= 1.0 {
                self.food_pulses.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Может ли магазин зарабатывать: есть касса и источник еды (ящик/стеллаж/
    /// витрина с остатком). Если нет объектов с едой или кассы — банкротство
    /// неизбежно, так как покупатели не могут ничего купить.
    fn shop_can_earn(&self, ecs: &crate::EcsAdapter) -> bool {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let mut has_cassa = false;
        for tag in (&tags).join() {
            if tag.name == "cassa" {
                has_cassa = true;
                break;
            }
        }
        if !has_cassa {
            return false;
        }
        let foods = ecs.world.read_storage::<FoodStorage>();
        let mut has_food = false;
        for (tag, food) in (&tags, &foods).join() {
            if tag.name == "box"
                || (tag.name == "rack" && food.food_count > 0)
                || (tag.name == "candies" && food.food_count > 0)
            {
                has_food = true;
                break;
            }
        }
        has_food
    }

    /// Собрать содержимое каталога магазина: для каждого светящегося
    /// предмета (INV_LIGHT) — имя, иконка, цена из баланса и флаг,
    /// куплен ли уже доступ в текущем мире (ShopOwned).
    fn shop_items(&self, ecs: &crate::EcsAdapter) -> Vec<crate::ui::shop::ShopItem> {
        let owned = &ecs.world.read_resource::<ShopOwned>().0;
        INV_LIGHT.iter().map(|n| {
            let is_owned = owned.iter().any(|o| o.as_str() == *n);
            let price = crate::data::object_price(n, &self.config);
            let icon = crate::core::util::slot_icon_path(n);
            ((*n).to_string(), icon, price, is_owned)
        }).collect()
    }

    /// Минимальная сумма, чтобы снова сделать магазин зарабатывающим:
    /// цена отсутствующей кассы + цена самого дешёвого источника еды (box).
    /// Если оба уже есть — 0 (восстановление не нужно).
    fn recovery_cost(ecs: &crate::EcsAdapter, cfg: &crate::scripts::config::BalanceConfig) -> i32 {
        let tags = ecs.world.read_storage::<ObjectTag>();
        let foods = ecs.world.read_storage::<FoodStorage>();
        let has_cassa = (&tags).join().any(|t| t.name == "cassa");
        let has_food = (&tags, &foods).join().any(|(t, f)| {
            t.name == "box" || (t.name == "rack" && f.food_count > 0) || (t.name == "candies" && f.food_count > 0)
        });
        let mut cost = 0;
        if !has_cassa {
            cost += crate::data::object_price("cassa", cfg);
        }
        if !has_food {
            cost += crate::data::object_price("box", cfg);
        }
        cost
    }

    /// Пульс при смене активного слота/режима: рамка и иконка «подпрыгивают».
    /// Устанавливает пульс, если слот/режим изменились с прошлого кадра.
    fn update_slot_mode_pulse(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        if self.act_slot != self.prev_act_slot {
            self.prev_act_slot = self.act_slot;
            self.slot_pulse = UI_PULSE_SECS;
        }
        if self.mode != self.prev_mode {
            self.prev_mode = self.mode;
            self.mode_pulse = UI_PULSE_SECS;
        }
        if self.slot_pulse > 0.0 {
            self.slot_pulse -= dt;
            let p = 1.0 - (self.slot_pulse / UI_PULSE_SECS).clamp(0.0, 1.0);
            let scale = 1.0 + 0.3 * (std::f32::consts::PI * p as f32).sin();
            if let Some(e) = self.icons_slot_cursor {
                ecs.update_sprite_scale(e, scale);
            }
        } else if let Some(e) = self.icons_slot_cursor {
            ecs.update_sprite_scale(e, 1.0);
        }
        if self.mode_pulse > 0.0 {
            self.mode_pulse -= dt;
            let p = 1.0 - (self.mode_pulse / UI_PULSE_SECS).clamp(0.0, 1.0);
            let scale = 1.0 + 0.3 * (std::f32::consts::PI * p as f32).sin();
            if let Some(e) = self.icon_mode {
                ecs.update_sprite_scale(e, scale);
            }
        } else if let Some(e) = self.icon_mode {
            ecs.update_sprite_scale(e, 1.0);
        }
    }

    /// Сбрасывает активные эффекты (мир очищается при смене уровня).
    pub fn clear_food_fx(&mut self) {
        self.food_pulses.clear();
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
        *ecs.world.write_resource::<crate::scripts::config::BalanceConfig>() = self.config.clone();
        self.last_frame = std::time::Instant::now();
        self.anim_timer = 0.0;
        self.camera_offset_x = 0.0;
        self.camera_offset_y = 0.0;
        self.ilm_entity = None;
        self.ilm_timer = 0.0;
        self.ilm_cooldown = 0.0;
        self.no_money_entity = None;
        self.no_money_timer = 0.0;
        self.food_timer = 0.0;
        self.active = true;
        self.active_entity = None;
        self.inv_entity = None;
        self.settings_entity = None;
        self.settings = crate::ui::settings::Settings::new();
        self.hud.reset();
        self.shoppers.clear();
        self.day_night.reset();
        self.food_pulses.clear();
        ecs.world.write_resource::<TotalFood>().0 = 0;
        ecs.world.write_resource::<BusyCassas>().0.clear();
        ecs.world.write_resource::<Money>().0 = self.config.start_money;
        self.rent_timer = 0.0;
        self.bankrupt = false;
        self.bankrupt_bg = None;
        self.bankrupt_title = None;
        self.bankrupt_hint = None;
        self.bankrupt_button = None;
        self.bankrupt_button_label = None;
        self.slot_pulse = 0.0;
        self.mode_pulse = 0.0;
        self.prev_act_slot = 0;
        self.prev_mode = 0;
        crate::audio::play_music("music");
    }

    /// Автосохранение мира при выходе из игры (в меню или закрытие приложения).
    fn on_exit(&mut self, ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer) {
        if self.shop.open {
            self.shop.close(ecs);
        }
        self.shop_entity = None;
        if let Some(id) = self.world_id {
            self.save_to_disk(ecs, id);
            crate::save::touch_world(id);
        }
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &dyn InputSource, window_size: (f32, f32), text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        // Адаптивный масштаб UI под соотношение сторон (хотбар + иконки/логотип
        // справа умещаются в экран даже на портретных/узких дисплеях)
        let aspect = if window_size.1 > 0.0 { window_size.0 / window_size.1 } else { 1.0 };
        self.ui_scale = crate::core::util::ui_fit_scale(aspect, 6.5);

        // Отложенная загрузка: показываем "Loading..." на один кадр, затем строим контент
        if !self.loaded {
            if self.loading {
                self.loading = false;
                self.show_loading(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
            self.hide_loading(ecs);
            self.loaded = true;
            // Какой мир запустить — берём из глобальной статики (выставляет меню)
            let selection = {
                let g = crate::save::SELECTED_WORLD.lock().unwrap();
                g.clone()
            };
            *crate::save::SELECTED_WORLD.lock().unwrap() = crate::save::WorldSelection::None;
            match selection {
                crate::save::WorldSelection::New(id, name) => {
                    self.world_id = Some(id);
                    self.world_name = name;
                    self.setup_content(ecs, text_renderer, device, queue);
                }
                crate::save::WorldSelection::Load(id) => {
                    self.world_id = Some(id);
                    self.world_name = crate::save::world_meta(id)
                        .map(|m| m.name)
                        .unwrap_or_else(|| format!("Мир {}", id));
                    if !self.load_from_disk(ecs, text_renderer, device, queue, id) {
                        self.setup_content(ecs, text_renderer, device, queue);
                    }
                }
                crate::save::WorldSelection::None => {
                    self.setup_content(ecs, text_renderer, device, queue);
                }
            }
        }

        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;

        // --- Аренда магазина (экономическая нагрузка) ---
        if !self.bankrupt {
            self.rent_timer += dt;
            if self.rent_timer >= self.config.rent_interval_secs {
                self.rent_timer = 0.0;
                let money = ecs.world.read_resource::<Money>().0;
                let paid = money.min(self.config.rent_amount);
                ecs.world.write_resource::<Money>().0 -= paid;
            }
        }

        // --- Банкротство: магазин не может зарабатывать и на восстановление не хватает денег ---
        if !self.bankrupt && self.current_level == 0 && !self.settings.open {
            if !self.shop_can_earn(ecs) {
                let money = ecs.world.read_resource::<Money>().0;
                let recovery = Self::recovery_cost(ecs, &self.config);
                if money < recovery {
                    self.bankrupt = true;
                    self.shoppers.set_active(false);
                    crate::audio::play("error");
                    let bg = ecs.add_ui_sized(0.0, 0.0, 24.0, 16.0, "assets/tex/dev_tools/black.png", device, queue);
                    ecs.update_sprite_alpha(bg, 0.75);
                    self.bankrupt_bg = Some(bg);
                    let title = text_renderer.add_text(ecs, device, queue, "BANKRUPT", FONT_SIZE_LOGO, 0.0, 2.0, 7.0, 1.0, RED);
                    self.bankrupt_title = Some(title);
                    let hint = text_renderer.add_text(ecs, device, queue, "Tap the button to return to menu", FONT_SIZE_BTN, 0.0, 0.5, 9.0, 1.0, WHITE);
                    self.bankrupt_hint = Some(hint);
                    // Кликабельная кнопка «В меню» (на телефоне нет клавиши R).
                    let btn = ecs.add_ui_sized(crate::core::constants::BANKRUPT_BTN_X, crate::core::constants::BANKRUPT_BTN_Y, crate::core::constants::BANKRUPT_BTN_HALF_W * 2.0, crate::core::constants::BANKRUPT_BTN_HALF_H * 2.0, "assets/tex/dev_tools/black.png", device, queue);
                    ecs.update_sprite_alpha(btn, 0.85);
                    self.bankrupt_button = Some(btn);
                    let btn_label = text_renderer.add_text(ecs, device, queue, "Menu", FONT_SIZE_BTN, crate::core::constants::BANKRUPT_BTN_X, crate::core::constants::BANKRUPT_BTN_Y, 3.0, 1.0, WHITE);
                    self.bankrupt_button_label = Some(btn_label);
                }
            }
        }
        if self.bankrupt {
            if input.key_pressed(KeyCode::KeyR) {
                return SceneAction::Switch("menu".to_string());
            }
            // Кнопка «В меню» для телефонов (нет клавиши R): клик/тап по ней.
            if let Some((mx, my)) = input.cursor() {
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                if input.mouse_pressed(winit::event::MouseButton::Left)
                    && (wx - crate::core::constants::BANKRUPT_BTN_X).abs() <= crate::core::constants::BANKRUPT_BTN_HALF_W
                    && (wy - crate::core::constants::BANKRUPT_BTN_Y).abs() <= crate::core::constants::BANKRUPT_BTN_HALF_H {
                    return SceneAction::Switch("menu".to_string());
                }
            }
            return SceneAction::None;
        }

        // --- Toggle settings / close shop ---
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            if self.shop.open {
                self.shop.close(ecs);
            } else if self.settings.open {
                self.settings.close(ecs);
            } else {
                self.settings.open(ecs, text_renderer, device, queue);
            }
            crate::audio::play("click");
        }

        // Кнопка настроек (иконка gear) — открыть/закрыть окно настроек
        // (неактивна, пока открыт магазин)
        if !self.shop.open {
            if let Some((mx, my)) = input.cursor() {
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                if input.mouse_pressed(winit::event::MouseButton::Left)
                    && (wx - SETTINGS_BTN_X).abs() < TILE_HALF
                    && (wy - SETTINGS_BTN_Y).abs() < TILE_HALF {
                    if self.settings.open {
                        self.settings.close(ecs);
                    } else {
                        self.settings.open(ecs, text_renderer, device, queue);
                    }
                    crate::audio::play("click");
                    return SceneAction::None;
                }
            }
        }

        // Кнопка магазина (правее инвентаря) — открыть/закрыть окно магазина
        // (неактивна, пока открыты настройки)
        if !self.settings.open {
            if let Some((mx, my)) = input.cursor() {
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                if input.mouse_pressed(winit::event::MouseButton::Left)
                    && (wx - SHOP_BTN_X).abs() < TILE_HALF
                    && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                    if self.shop.open {
                        self.shop.close(ecs);
                    } else {
                        if self.inventory.open {
                            self.inventory.exit(ecs);
                        }
                        let items = self.shop_items(ecs);
                        self.shop.open(ecs, text_renderer, device, queue, &items);
                    }
                    crate::audio::play("click");
                    return SceneAction::None;
                }
            }
        }

        // --- Ручное сохранение (Ctrl+S) в текущий мир ---
        if input.held_control() && input.key_pressed(KeyCode::KeyS) {
            if let Some(id) = self.world_id {
                self.save_to_disk(ecs, id);
                crate::save::touch_world(id);
            }
        }

        if self.settings.open {
            // Hover-анимация настроек (масштаб галочки/ползунка)
            self.settings.tick_hover(ecs, input, window_size, dt);
            // Закрытие по клику/тапу вне панели настроек (ПК и Android/тач).
            // Панель центрирована в (0,0) с размерами w×h, поэтому «снаружи» —
            // это выход за её половинные границы.
            if let Some((mx, my)) = input.cursor() {
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                let p = &self.settings.panel;
                let inside = (wx - p.x).abs() <= p.w / 2.0 && (wy - p.y).abs() <= p.h / 2.0;
                if input.mouse_pressed(winit::event::MouseButton::Left) && !inside {
                    self.settings.close(ecs);
                    crate::audio::play("click");
                    return SceneAction::None;
                }
            }
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
            if self.settings.menu_requested {
                self.settings.menu_requested = false;
                return SceneAction::Switch("menu".to_string());
            }
        } else if self.shop.open {
            // --- Магазин открыт: клики только по каталогу ---
            if let Some(idx) = self.shop.row_clicked(input, window_size) {
                let name = INV_LIGHT[idx].to_string();
                let owned = ecs.world.read_resource::<ShopOwned>().0.iter().any(|o| o == &name);
                if !owned {
                    let price = crate::data::object_price(&name, &self.config);
                    let money = ecs.world.read_resource::<Money>().0;
                    if money >= price {
                        ecs.world.write_resource::<Money>().0 = money - price;
                        ecs.world.write_resource::<ShopOwned>().0.push(name.clone());
                        crate::audio::play("click");
                        let items = self.shop_items(ecs);
                        self.shop.refresh(ecs, text_renderer, device, queue, &items);
                    } else {
                        // Недостаточно денег — покажем красную подсказку
                        ecs.world.write_resource::<PlacementError>().0 = Some((money, price));
                    }
                }
                return SceneAction::None;
            }
            // Закрытие по клику вне панели магазина
            if let Some((mx, my)) = input.cursor() {
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                let p = &self.shop.panel;
                let inside = (wx - p.x).abs() <= p.w / 2.0 && (wy - p.y).abs() <= p.h / 2.0;
                if input.mouse_pressed(winit::event::MouseButton::Left) && !inside {
                    self.shop.close(ecs);
                    crate::audio::play("click");
                    return SceneAction::None;
                }
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
                    let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
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
                    let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                    // Кнопка "открыт/закрыт": переключает доступность магазина для покупателей
                    if (wx - ACTIVE_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        self.active = !self.active;
                        let tex = if self.active { TEX_ACTIVE } else { TEX_NO_ACTIVE };
                        if let Some(entity) = self.active_entity {
                            ecs.update_sprite_texture(entity, tex);
                        }
                        self.shoppers.set_active(self.active);
                        crate::audio::play("bell");
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
                crate::audio::play("stair");
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

        // --- Всплывающие подсказки (работают во всех режимах) ---
        // Не хватает денег: PlacementError содержит (деньги, цена).
        let placement_err = ecs.world.read_resource::<PlacementError>().0;
        if let Some((money, price)) = placement_err {
            ecs.world.write_resource::<PlacementError>().0 = None;
            if self.no_money_entity.is_none() {
                let msg = format!("Not enough money: you have {}, but it costs {}", money, price);
                let ent = text_renderer.add_text(ecs, device, queue, &msg, 40.0, 0.0, -3.0, 8.0, 1.0, RED);
                self.no_money_entity = Some(ent);
                self.no_money_timer = 2.0;
            }
        }
        // Светящийся предмет не куплен: ShopDenied выставляется при попытке
        // поставить объект, доступ к которому покупается в магазине.
        let shop_denied = ecs.world.read_resource::<ShopDenied>().0;
        if shop_denied {
            ecs.world.write_resource::<ShopDenied>().0 = false;
            if self.no_money_entity.is_none() {
                let ent = text_renderer.add_text(ecs, device, queue, "Buy this in the Shop", 40.0, 0.0, -3.0, 8.0, 1.0, RED);
                self.no_money_entity = Some(ent);
                self.no_money_timer = 2.0;
            }
        }

        // Прогресс дня/ночи в независимости от режима настроек
        self.day_night.tick(dt);

        self.update_camera(input, window_size, dt);

        // --- Обновление всех объектов по компонентам ---
        // Периодическая регенерация еды в ящиках (box)
        self.food_timer += dt;
        if self.food_timer >= self.config.food_regen_tick {
            self.food_timer -= self.config.food_regen_tick;
            let amount = self.config.food_regen_amount;
            let mut adds = Vec::new();
            {
                let tags = ecs.world.read_storage::<ObjectTag>();
                let mut foods = ecs.world.write_storage::<FoodStorage>();
                let groups = ecs.world.read_storage::<crate::GroupComponent>();
                for (tag, storage, group) in (&tags, &mut foods, &groups).join() {
                    if tag.name == "box" && storage.food_count < storage.max_food {
                        storage.food_count += amount;
                        adds.push(group.group_id);
                    }
                }
            }
            ecs.update_object_textures();
            ecs.pending_food_adds.extend(adds);
        }
        // Обрабатываем накопленные эффекты появления еды (ящики и стеллажи)
        self.drain_food_fx(ecs);
        self.update_food_fx(ecs, dt);
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
                let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
                let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
                let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
                if col >= 0 && col < INVENTORY_COLS && row >= 0 && row < INVENTORY_ROWS {
                    let item_idx = crate::core::util::inventory_index(row, col) as usize;
                    let items = self.inventory.items();
                    if item_idx < items.len() {
                        let name = items[item_idx];
                        let price = crate::data::object_price(name, &self.config);
                        Some((name.to_string(), price, SLOT_BAR_X + col as f32, INVENTORY_BASE_Y + row as f32 - 0.55))
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

        // Управление таймером красной подсказки «не хватает денег»
        if let Some(ent) = self.no_money_entity {
            self.no_money_timer -= dt;
            if self.no_money_timer <= 0.0 {
                ecs.delete_entity(ent);
                self.no_money_entity = None;
            }
        }

        // --- Shopper NPCs ---
        self.shoppers.tick(ecs, dt, &self.npc_walkable, self.active, &self.config, self.npc_script.as_ref());

        self.update_animations(ecs, dt);

        // --- UI-анимации: пульсы счётчиков, фейды подсказок, поп инвентаря ---
        self.hud.tick(ecs, dt);
        self.inventory.tick(ecs, dt);

        // --- Пульс активного слота и иконки режима при их смене ---
        self.update_slot_mode_pulse(ecs, dt);

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter, visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        // Отдаём слои рендера с отсечением по видимой области
        ecs.get_sprites_by_layer(visible_bounds)
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }

    fn ui_size(&self) -> f32 {
        self.ui_scale
    }

    fn camera_offset(&self) -> (f32, f32) {
        (self.camera_offset_x, self.camera_offset_y)
    }

    /// Коэффициент затенения для шейдера (ночь затемняет мир)
    fn night_factor(&self) -> f32 {
        self.day_night.factor(&self.config)
    }

    fn lights(&self, ecs: &crate::EcsAdapter) -> Vec<crate::core::buffers::LightData> {
        let transforms = ecs.world.read_storage::<crate::ecs::components::Transform>();
        let lights = ecs.world.read_storage::<crate::ecs::components::PointLight>();
        
        (&transforms, &lights).join()
            .take(crate::core::constants::MAX_LIGHTS)
            .map(|(t, l)| {
                crate::core::buffers::LightData {
                    position: [t.position[0], t.position[1], t.position[2], 0.0],
                    color: [l.color[0], l.color[1], l.color[2], l.intensity],
                    radius: l.radius,
                    _padding: [0.0; 7],
                }
            })
            .collect()
    }
}
