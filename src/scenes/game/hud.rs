// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use crate::core::constants::*;
use crate::EcsAdapter;
use crate::ui::text_renderer::TextRenderer;

// ========================================================================
//  GameHud — игровой интерфейс (надписи поверх карты)
// ========================================================================
//  Управляет текстовыми сущностями HUD: "Loading...", подсказка над
//  объектом, запасы еды, деньги, часы, тултип предмета в инвентаре и
//  инфо-панель. Тексты пересоздаются через set_text только когда их
//  содержимое реально изменилось — так кэш спрайтов не засоряется.

pub struct GameHud {
    loading_text: Option<specs::Entity>,
    loading_key: Option<u64>,
    // Подпись "имя: food/max" над наведённым объектом
    object_hover_text: Option<specs::Entity>,
    object_hover_key: Option<u64>,
    // Счётчик еды в углу экрана
    total_food_text: Option<specs::Entity>,
    total_food_key: Option<u64>,
    current_total_food: i32,
    // Кол-во денег в углу экрана
    money_text: Option<specs::Entity>,
    money_key: Option<u64>,
    current_money: i32,
    // Текущее игровое время (день/ночь)
    time_text: Option<specs::Entity>,
    time_key: Option<u64>,
    current_time_string: String,
    time_update_timer: f64,
    // Тултип над ячейкой инвентаря (имя предмета + подложка)
    slot_tooltip_text: Option<specs::Entity>,
    slot_tooltip_text_key: Option<u64>,
    slot_tooltip_bg: Option<specs::Entity>,
    slot_tooltip_bg_key: Option<u64>,
    info_panel: Option<specs::Entity>,
    // Пульс счётчиков денег/еды при изменении значения: таймер пульса
    money_pulse: f64,
    food_pulse: f64,
    // Фейд подсказок: текущая и целевая альфа (1 — показать, 0 — скрыть)
    hover_alpha: f32,
    hover_target: f32,
    tooltip_alpha: f32,
    tooltip_target: f32,
}

// Пульс счётчиков денег/еды и фейд подсказок используют общие константы
// Пульс счётчиков денег/еды и фейд подсказок используют общие константы
// UI_PULSE_SECS / UI_FADE_SPEED из src/core/constants.rs.

impl GameHud {
    pub fn new() -> Self {
        GameHud {
            loading_text: None,
            loading_key: None,
            object_hover_text: None,
            object_hover_key: None,
            total_food_text: None,
            total_food_key: None,
            current_total_food: -1,
            money_text: None,
            money_key: None,
            current_money: -1,
            time_text: None,
            time_key: None,
            current_time_string: String::new(),
            time_update_timer: 1.0,
            slot_tooltip_text: None,
            slot_tooltip_text_key: None,
            slot_tooltip_bg: None,
            slot_tooltip_bg_key: None,
            info_panel: None,
            money_pulse: 0.0,
            food_pulse: 0.0,
            hover_alpha: 0.0,
            hover_target: 0.0,
            tooltip_alpha: 0.0,
            tooltip_target: 0.0,
        }
    }

    /// Сбрасывает все ссылки на сущности/кэш (вызывается при загрузке уровня).
    /// Сами сущности очищаются миром, здесь мы только забываем ссылки.
    pub fn reset(&mut self) {
        self.loading_text = None;
        self.loading_key = None;
        self.object_hover_text = None;
        self.object_hover_key = None;
        self.total_food_text = None;
        self.total_food_key = None;
        self.current_total_food = -1;
        self.money_text = None;
        self.money_key = None;
        self.current_money = -1;
        self.time_text = None;
        self.time_key = None;
        self.current_time_string.clear();
        self.time_update_timer = 1.0;
        self.slot_tooltip_text = None;
        self.slot_tooltip_text_key = None;
        self.slot_tooltip_bg = None;
        self.slot_tooltip_bg_key = None;
        self.info_panel = None;
        self.money_pulse = 0.0;
        self.food_pulse = 0.0;
        self.hover_alpha = 0.0;
        self.hover_target = 0.0;
        self.tooltip_alpha = 0.0;
        self.tooltip_target = 0.0;
    }

    // ====================================================================
    //  Загрузка
    // ====================================================================

    /// Показывает надпись "Loading..." в центре экрана
    pub fn show_loading(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let (entity, key) = text_renderer.set_text(ecs, device, queue, self.loading_text, self.loading_key, "Loading...", 64.0, 0.0, 0.0, 4.0, 2.0, GRAY);
        self.loading_text = entity;
        self.loading_key = key;
    }

    /// Убирает надпись "Loading..." по завершении загрузки
    pub fn hide_loading(&mut self, ecs: &mut EcsAdapter) {
        if let Some(entity) = self.loading_text.take() {
            ecs.delete_entity(entity);
        }
        if let Some(key) = self.loading_key.take() {
            ecs.sprite_cache.remove(&key);
        }
    }

    /// Создаёт полупрозрачную тёмную панель-подложку (угол экрана)
    pub fn create_info_panel(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        let panel = ecs.add_ui_sized(5.75, 3.25, 1.2, 2.2, "tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(panel, 0.5);
        self.info_panel = Some(panel);
    }

    // ====================================================================
    //  Обновление текста по кадрам
    // ====================================================================

    /// Показывает/скрывает подсказку над наведённым объектом.
    /// Появление/скрытие плавное: альфа меняется в tick().
    pub fn update_hover(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, hovered: Option<(i32, i32, String)>) {
        match hovered {
            Some((food, max, name)) => {
                let text = format!("{}: {}/{}", name, food, max);
                let (entity, key) = text_renderer.set_text(ecs, device, queue, self.object_hover_text, self.object_hover_key, &text, 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
                self.object_hover_text = entity;
                self.object_hover_key = key;
                // Только начали показывать — стартуем с нулевой прозрачности.
                if let Some(e) = entity {
                    ecs.update_sprite_alpha(e, self.hover_alpha);
                }
                self.hover_target = 1.0;
            }
            None => {
                // Не удаляем сразу: tick() плавно гасит альфу и удаляет у цели.
                self.hover_target = 0.0;
            }
        }
    }

    /// Плавное появление/скрытие подсказки над объектом (вызывается каждый кадр).
    fn tick_hover(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        let step = UI_FADE_SPEED * dt as f32;
        if self.hover_target > self.hover_alpha {
            self.hover_alpha = (self.hover_alpha + step).min(self.hover_target);
        } else if self.hover_alpha > self.hover_target {
            self.hover_alpha = (self.hover_alpha - step * 0.7).max(self.hover_target);
        }
        if let Some(entity) = self.object_hover_text {
            ecs.update_sprite_alpha(entity, self.hover_alpha);
            // Достигли нуля — полностью скрываем и чистим спрайт из кэша.
            if self.hover_target == 0.0 && self.hover_alpha <= 0.01 {
                self.object_hover_text = None;
                ecs.delete_entity(entity);
                if let Some(key) = self.object_hover_key.take() {
                    ecs.sprite_cache.remove(&key);
                }
            }
        }
    }

    /// Обновляет счётчики еды и денег, только если они изменились.
    /// При изменении значения запускается пульс (кнопка «подпрыгивает»).
    pub fn update_stats(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, total_food: i32, money: i32) {
        if total_food != self.current_total_food {
            self.current_total_food = total_food;
            self.food_pulse = UI_PULSE_SECS;
            let text = format!("Food: {}", total_food);
            let (entity, key) = text_renderer.set_text(ecs, device, queue, self.total_food_text, self.total_food_key, &text, 64.0, 5.75, 3.5, 1.0, 4.0, WHITE);
            self.total_food_text = entity;
            self.total_food_key = key;
        }
        if money != self.current_money {
            self.current_money = money;
            self.money_pulse = UI_PULSE_SECS;
            let text = format!("Money: {}", money);
            let (entity, key) = text_renderer.set_text(ecs, device, queue, self.money_text, self.money_key, &text, 64.0, 5.75, 3.0, 1.0, 4.0, WHITE);
            self.money_text = entity;
            self.money_key = key;
        }
    }

    /// Пульс счётчиков: прозрачность плавно 1.0 → 0.55 → 1.0 (вызывается каждый кадр).
    /// Используем альфу, а не масштаб: текст-спрайты нельзя пересоздать из пути,
    /// а альфа не участвует в ключе кэша спрайтов.
    fn tick_stats_pulse(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.money_pulse > 0.0 {
            self.money_pulse -= dt;
            let p = 1.0 - (self.money_pulse / UI_PULSE_SECS).clamp(0.0, 1.0);
            let alpha = 1.0 - 0.45 * (std::f32::consts::PI * p as f32).sin();
            if let Some(e) = self.money_text {
                ecs.update_sprite_alpha(e, alpha);
            }
        } else if let Some(e) = self.money_text {
            ecs.update_sprite_alpha(e, 1.0);
        }
        if self.food_pulse > 0.0 {
            self.food_pulse -= dt;
            let p = 1.0 - (self.food_pulse / UI_PULSE_SECS).clamp(0.0, 1.0);
            let alpha = 1.0 - 0.45 * (std::f32::consts::PI * p as f32).sin();
            if let Some(e) = self.total_food_text {
                ecs.update_sprite_alpha(e, alpha);
            }
        } else if let Some(e) = self.total_food_text {
            ecs.update_sprite_alpha(e, 1.0);
        }
    }

    /// Обновляет часы. Перерисовка выполняется не чаще раза в секунду,
    /// чтобы строка времени не мигала и не спамила кэш.
    pub fn update_time(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, time_str: &str, dt: f64) {
        self.time_update_timer += dt;
        if time_str != self.current_time_string && self.time_update_timer >= 1.0 {
            self.time_update_timer = 0.0;
            self.current_time_string = time_str.to_string();
            let (entity, key) = text_renderer.set_text(ecs, device, queue, self.time_text, self.time_key, time_str, 64.0, 5.75, 2.5, 1.0, 4.0, WHITE);
            self.time_text = entity;
            self.time_key = key;
        }
    }

    /// Показывает/скрывает тултип с названием предмета над ячейкой инвентаря.
    /// Подложка создаётся один раз под размер текста, далее только двигается.
    /// Появление/скрытие плавное через альфу в tick_tooltip().
    pub fn update_slot_tooltip(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, tooltip: Option<(String, f32, f32)>) {
        match tooltip {
            Some((name, tx, ty)) => {
                let display_name = name.replace('_', " ");
                let char_w = 0.14;
                let text_w = (display_name.len() as f32).max(1.0) * char_w;
                let (_, text_h) = text_renderer.text_world_size(&display_name, FONT_SIZE_LOGO, text_w, 4.0, WHITE);
                // Подложка чуть больше текста по обеим осям
                let pad_x = 0.10;
                let pad_y = 0.04;
                let bg_w = text_w + pad_x * 2.0;
                let bg_h = text_h + pad_y * 2.0;

                // Ключ кэша уникален для каждого размера подложки
                let bg_unique = format!("tex/dev_tools/black.png@{bg_w:.2}x{bg_h:.2}");
                let bg_key = crate::core::util::sprite_cache_key("ui", &bg_unique, [0, 0], [1, 1], 1.0);
                if self.slot_tooltip_bg_key != Some(bg_key) {
                    // Размер изменился — пересоздаём подложку
                    if let Some(old) = self.slot_tooltip_bg.take() {
                        ecs.delete_entity(old);
                    }
                    if let Some(old_key) = self.slot_tooltip_bg_key.take() {
                        ecs.sprite_cache.remove(&old_key);
                    }
                    let bg_ent = ecs.add_ui_sized(tx, ty, bg_w, bg_h, "tex/dev_tools/black.png", device, queue);
                    ecs.update_sprite_alpha(bg_ent, self.tooltip_alpha * 0.5);
                    self.slot_tooltip_bg = Some(bg_ent);
                    self.slot_tooltip_bg_key = Some(bg_key);
                } else if let Some(entity) = self.slot_tooltip_bg {
                    // Размер тот же — просто переносим подложку к курсору
                    ecs.update_transform_position(entity, tx, ty);
                }

                let (entity, key) = text_renderer.set_text(ecs, device, queue, self.slot_tooltip_text, self.slot_tooltip_text_key, &display_name, FONT_SIZE_LOGO, tx, ty, text_w, 4.0, WHITE);
                if let Some(e) = entity {
                    ecs.update_sprite_alpha(e, self.tooltip_alpha);
                }
                self.slot_tooltip_text = entity;
                self.slot_tooltip_text_key = key;
                self.tooltip_target = 1.0;
            }
            None => {
                // Не удаляем сразу: tick_tooltip() плавно гасит альфу и удаляет.
                self.tooltip_target = 0.0;
            }
        }
    }

    /// Плавное появление/скрытие тултипа инвентаря (вызывается каждый кадр).
    fn tick_tooltip(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        let step = UI_FADE_SPEED * dt as f32;
        if self.tooltip_target > self.tooltip_alpha {
            self.tooltip_alpha = (self.tooltip_alpha + step).min(self.tooltip_target);
        } else if self.tooltip_alpha > self.tooltip_target {
            self.tooltip_alpha = (self.tooltip_alpha - step * 0.7).max(self.tooltip_target);
        }
        if let Some(entity) = self.slot_tooltip_text {
            ecs.update_sprite_alpha(entity, self.tooltip_alpha);
        }
        if let Some(entity) = self.slot_tooltip_bg {
            ecs.update_sprite_alpha(entity, self.tooltip_alpha * 0.5);
        }
        // Полностью скрыт — удаляем текст, подложку и ключи из кэша.
        if self.tooltip_target == 0.0 && self.tooltip_alpha <= 0.01 {
            if let Some(entity) = self.slot_tooltip_text.take() {
                ecs.delete_entity(entity);
            }
            if let Some(key) = self.slot_tooltip_text_key.take() {
                ecs.sprite_cache.remove(&key);
            }
            if let Some(entity) = self.slot_tooltip_bg.take() {
                ecs.delete_entity(entity);
            }
            if let Some(key) = self.slot_tooltip_bg_key.take() {
                ecs.sprite_cache.remove(&key);
            }
        }
    }

    /// Ежекадровый тик анимаций HUD: пульсы счётчиков и фейды подсказок.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        self.tick_stats_pulse(ecs, dt);
        self.tick_hover(ecs, dt);
        self.tick_tooltip(ecs, dt);
    }
}