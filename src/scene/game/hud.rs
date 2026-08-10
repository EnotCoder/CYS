use crate::constants::*;
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
}

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

    /// Показывает/скрывает подсказку над наведённым объектом
    pub fn update_hover(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, hovered: Option<(i32, i32, String)>) {
        match hovered {
            Some((food, max, name)) => {
                let text = format!("{}: {}/{}", name, food, max);
                let (entity, key) = text_renderer.set_text(ecs, device, queue, self.object_hover_text, self.object_hover_key, &text, 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
                self.object_hover_text = entity;
                self.object_hover_key = key;
            }
            None => {
                // Курсор ушёл с объекта — удаляем подсказку
                if let Some(entity) = self.object_hover_text.take() {
                    ecs.delete_entity(entity);
                }
                if let Some(key) = self.object_hover_key.take() {
                    ecs.sprite_cache.remove(&key);
                }
            }
        }
    }

    /// Обновляет счётчики еды и денег, только если они изменились
    pub fn update_stats(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, total_food: i32, money: i32) {
        if total_food != self.current_total_food {
            self.current_total_food = total_food;
            let text = format!("Food: {}", total_food);
            let (entity, key) = text_renderer.set_text(ecs, device, queue, self.total_food_text, self.total_food_key, &text, 64.0, 5.75, 3.5, 1.0, 4.0, WHITE);
            self.total_food_text = entity;
            self.total_food_key = key;
        }
        if money != self.current_money {
            self.current_money = money;
            let text = format!("Money: {}", money);
            let (entity, key) = text_renderer.set_text(ecs, device, queue, self.money_text, self.money_key, &text, 64.0, 5.75, 3.0, 1.0, 4.0, WHITE);
            self.money_text = entity;
            self.money_key = key;
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
                let bg_key = crate::util::sprite_cache_key("ui", &bg_unique, [0, 0], [1, 1], 1.0);
                if self.slot_tooltip_bg_key != Some(bg_key) {
                    // Размер изменился — пересоздаём подложку
                    if let Some(old) = self.slot_tooltip_bg.take() {
                        ecs.delete_entity(old);
                    }
                    if let Some(old_key) = self.slot_tooltip_bg_key.take() {
                        ecs.sprite_cache.remove(&old_key);
                    }
                    let bg_ent = ecs.add_ui_sized(tx, ty, bg_w, bg_h, "tex/dev_tools/black.png", device, queue);
                    ecs.update_sprite_alpha(bg_ent, 0.5);
                    self.slot_tooltip_bg = Some(bg_ent);
                    self.slot_tooltip_bg_key = Some(bg_key);
                } else if let Some(entity) = self.slot_tooltip_bg {
                    // Размер тот же — просто переносим подложку к курсору
                    ecs.update_transform_position(entity, tx, ty);
                }

                let (entity, key) = text_renderer.set_text(ecs, device, queue, self.slot_tooltip_text, self.slot_tooltip_text_key, &display_name, FONT_SIZE_LOGO, tx, ty, text_w, 4.0, WHITE);
                self.slot_tooltip_text = entity;
                self.slot_tooltip_text_key = key;
            }
            None => {
                // Курсор вне инвентаря — удаляем текст и подложку тултипа
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
    }
}