// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Окно настроек: чекбокс Vertical Sync и слайдер скорости зума.
//  Все элементы создаются только в открытом состоянии и уничтожаются при
//  закрытии, чтобы не занимать сущности/спрайты в ECS.
// ========================================================================

use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::text_renderer::TextRenderer;
use crate::ui::{
    button_clicked, checkbox_clicked, create_button, create_checkbox, create_panel, create_slider,
    destroy_button, destroy_checkbox, destroy_panel, destroy_slider, refresh_checkbox, slider_drag,
    update_button_hover, update_checkbox_hover, update_slider_hover, update_slider_thumb, Button,
    Checkbox, Panel, Slider,
};
use crate::EcsAdapter;
use specs::Entity;

/// Состояние окна настроек и его UI-элементы.
pub struct Settings {
    /// true, пока окно открыто.
    pub open: bool,
    pub panel: Panel,
    /// Заголовок окна (текстовый спрайт "Settings").
    pub title: Option<Entity>,
    pub vsync: Checkbox,
    /// Флаг, что vsync изменился — сцена вернёт SceneAction
    pub vsync_toggled: bool,
    /// Слайдер скорости зума
    pub zoom_speed: Slider,
    /// Флаг, что значение слайдера изменилось в этом кадре.
    pub zoom_speed_changed: bool,
    /// Чекбокс «Музыка» (фоновая мелодия)
    pub music: Checkbox,
    /// Чекбокс «Звуковые эффекты» (клики, касса, погода)
    pub sfx: Checkbox,
    /// Флаги, что переключатели звука изменены в этом кадре.
    pub music_toggled: bool,
    pub sfx_toggled: bool,
    /// Выбор текстур-пака: стрелки, подложка значения, подпись и текущее значение.
    pack_prev: Button,
    pack_next: Button,
    pack_value_bg: Panel,
    pack_label: Option<Entity>,
    pack_label_key: Option<u64>,
    pack_value: Option<Entity>,
    pack_value_key: Option<u64>,
    /// Все доступные паки: первый — Vanilla.
    pack_list: Vec<crate::core::texture_pack::PackInfo>,
    pack_index: usize,
    /// Флаг смены пака в этом кадре.
    pub texture_pack_changed: bool,
    /// Выбор шейдера: стрелки, подложка значения, подпись и текущее значение.
    shader_prev: Button,
    shader_next: Button,
    shader_value_bg: Panel,
    shader_label: Option<Entity>,
    shader_label_key: Option<u64>,
    shader_value: Option<Entity>,
    shader_value_key: Option<u64>,
    shader_list: Vec<crate::core::shader_pack::ShaderInfo>,
    shader_index: usize,
    /// Флаг смены шейдера в этом кадре.
    pub shader_changed: bool,
    /// Кнопка открытия окна модов
    mods_button: Button,
    /// Флаг запроса окна модов, устанавливается по клику на mods_button
    pub mods_requested: bool,
    /// Кнопка «В меню» (выход из игры в выбор миров)
    menu_button: Button,
    /// Показывать ли кнопку «В меню»: в главном меню она не нужна.
    show_menu_button: bool,
    /// Флаг запроса выхода в меню, устанавливается по клику на menu_button
    pub menu_requested: bool,
    anim: PanelAnim,
}

impl Settings {
    const SELECTOR_VALUE_X: f32 = 2.10;
    const SELECTOR_VALUE_W: f32 = 2.20;
    const SELECTOR_VALUE_H: f32 = 0.50;
    const SELECTOR_ARROW_SIZE: f32 = 0.45;
    const SELECTOR_LEFT_X: f32 = 0.70;
    const SELECTOR_RIGHT_X: f32 = 3.50;
    const PACK_ROW_Y: f32 = 0.10;
    const SHADER_ROW_Y: f32 = -0.60;

    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 9.0, 6.0, 0.85),
            title: None,
            vsync: Checkbox::new(-1.4, 1.6, "Vertical Sync", true),
            vsync_toggled: false,
            zoom_speed: Slider::new(-0.1, 0.8, "Zoom Speed", 0.02, 0.3, 0.1),
            zoom_speed_changed: false,
            pack_prev: Button::textured(
                Self::SELECTOR_LEFT_X,
                Self::PACK_ROW_Y,
                Self::SELECTOR_ARROW_SIZE,
                Self::SELECTOR_ARROW_SIZE,
                "",
                "assets/tex/ui/icon_slots/arrow_left.png",
            ),
            pack_next: Button::textured(
                Self::SELECTOR_RIGHT_X,
                Self::PACK_ROW_Y,
                Self::SELECTOR_ARROW_SIZE,
                Self::SELECTOR_ARROW_SIZE,
                "",
                "assets/tex/ui/icon_slots/arrow_right.png",
            ),
            pack_value_bg: Panel::new(
                Self::SELECTOR_VALUE_X,
                Self::PACK_ROW_Y,
                Self::SELECTOR_VALUE_W,
                Self::SELECTOR_VALUE_H,
                0.45,
            ),
            pack_label: None,
            pack_label_key: None,
            pack_value: None,
            pack_value_key: None,
            pack_list: crate::core::texture_pack::list(),
            pack_index: 0,
            texture_pack_changed: false,
            shader_prev: Button::textured(
                Self::SELECTOR_LEFT_X,
                Self::SHADER_ROW_Y,
                Self::SELECTOR_ARROW_SIZE,
                Self::SELECTOR_ARROW_SIZE,
                "",
                "assets/tex/ui/icon_slots/arrow_left.png",
            ),
            shader_next: Button::textured(
                Self::SELECTOR_RIGHT_X,
                Self::SHADER_ROW_Y,
                Self::SELECTOR_ARROW_SIZE,
                Self::SELECTOR_ARROW_SIZE,
                "",
                "assets/tex/ui/icon_slots/arrow_right.png",
            ),
            shader_value_bg: Panel::new(
                Self::SELECTOR_VALUE_X,
                Self::SHADER_ROW_Y,
                Self::SELECTOR_VALUE_W,
                Self::SELECTOR_VALUE_H,
                0.45,
            ),
            shader_label: None,
            shader_label_key: None,
            shader_value: None,
            shader_value_key: None,
            shader_list: crate::core::shader_pack::list(),
            shader_index: 0,
            shader_changed: false,
            music: Checkbox::new(-1.4, -1.30, "Game music", true),
            sfx: Checkbox::new(-1.4, -1.90, "Sound Effects", true),
            music_toggled: false,
            sfx_toggled: false,
            mods_button: Button::new(-1.5, -2.55, 2.6, 0.7, "Mods..."),
            mods_requested: false,
            menu_button: Button::new(1.6, -2.55, 2.6, 0.7, "Back to menu"),
            show_menu_button: true,
            menu_requested: false,
            anim: PanelAnim::new(),
        }
    }

    /// Скрывает кнопку «В меню» — используется, когда панель открыта из главного меню.
    pub fn hide_menu_button(&mut self) {
        self.show_menu_button = false;
    }

    /// Id выбранного пака; None означает ванильные текстуры.
    pub fn texture_pack_id(&self) -> Option<String> {
        self.pack_list
            .get(self.pack_index)
            .map(|p| p.id.clone())
            .filter(|id| !id.is_empty())
    }

    /// Перечитывает паки и ставит сохранённый выбор (или ваниль).
    fn refresh_pack_list(&mut self, saved: Option<&str>) {
        crate::core::texture_pack::rescan();
        self.pack_list = crate::core::texture_pack::list();
        let current = saved
            .map(str::to_owned)
            .or_else(crate::core::texture_pack::active_id);
        self.pack_index = current
            .as_deref()
            .and_then(|id| self.pack_list.iter().position(|p| p.id == id))
            .unwrap_or(0);
    }

    fn pack_name(&self) -> String {
        self.pack_list
            .get(self.pack_index)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Vanilla".to_string())
    }

    /// Перерисовывает подпись текущего пака.
    fn update_pack_value(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.pack_value,
            self.pack_value_key,
            &self.pack_name(),
            46.0,
            Self::SELECTOR_VALUE_X,
            Self::PACK_ROW_Y,
            1.95,
            1.4,
            WHITE,
        );
        self.pack_value = entity;
        self.pack_value_key = key;
    }

    fn cycle_pack(
        &mut self,
        delta: i32,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.pack_list.len() < 2 {
            return;
        }
        let n = self.pack_list.len() as i32;
        self.pack_index = (self.pack_index as i32 + delta).rem_euclid(n) as usize;
        self.update_pack_value(ecs, text_renderer, device, queue);
        self.texture_pack_changed = true;
        crate::audio::play("click");
    }

    /// Id выбранного шейдера; None означает встроенный WGSL.
    pub fn shader_id(&self) -> Option<String> {
        self.shader_list
            .get(self.shader_index)
            .map(|shader| shader.id.clone())
            .filter(|id| !id.is_empty())
    }

    fn refresh_shader_list(&mut self, saved: Option<&str>) {
        crate::core::shader_pack::rescan();
        self.shader_list = crate::core::shader_pack::list();
        let current = saved
            .map(str::to_owned)
            .or_else(crate::core::shader_pack::active_id);
        self.shader_index = current
            .as_deref()
            .and_then(|id| self.shader_list.iter().position(|shader| shader.id == id))
            .unwrap_or(0);
    }

    fn shader_value(&self) -> (String, [u8; 3]) {
        if let Some(error) = crate::core::shader_pack::error() {
            let flat = error.replace('\n', " ");
            let short: String = flat.chars().take(36).collect();
            let text = if flat.chars().count() > 36 {
                format!("Error: {short}…")
            } else {
                format!("Error: {flat}")
            };
            return (text, RED);
        }
        let name = self
            .shader_list
            .get(self.shader_index)
            .map(|shader| shader.name.clone())
            .unwrap_or_else(|| "Vanilla".to_string());
        (name, WHITE)
    }

    fn update_shader_value(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (text, color) = self.shader_value();
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.shader_value,
            self.shader_value_key,
            &text,
            44.0,
            Self::SELECTOR_VALUE_X,
            Self::SHADER_ROW_Y,
            1.95,
            1.2,
            color,
        );
        self.shader_value = entity;
        self.shader_value_key = key;
    }

    fn cycle_shader(
        &mut self,
        delta: i32,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.shader_list.len() < 2 {
            return;
        }
        let n = self.shader_list.len() as i32;
        self.shader_index = (self.shader_index as i32 + delta).rem_euclid(n) as usize;
        self.update_shader_value(ecs, text_renderer, device, queue);
        self.shader_changed = true;
        crate::audio::play("click");
    }

    fn collect_entities(&self) -> Vec<Entity> {
        let mut v = Vec::new();
        if let Some(e) = self.panel.entity {
            v.push(e);
        }
        if let Some(e) = self.title {
            v.push(e);
        }
        if let Some(e) = self.vsync.box_entity {
            v.push(e);
        }
        if let Some(e) = self.vsync.label_entity {
            v.push(e);
        }
        if let Some(e) = self.zoom_speed.track {
            v.push(e);
        }
        if let Some(e) = self.zoom_speed.thumb {
            v.push(e);
        }
        if let Some(e) = self.zoom_speed.label_entity {
            v.push(e);
        }
        if let Some(e) = self.music.box_entity {
            v.push(e);
        }
        if let Some(e) = self.music.label_entity {
            v.push(e);
        }
        if let Some(e) = self.sfx.box_entity {
            v.push(e);
        }
        if let Some(e) = self.sfx.label_entity {
            v.push(e);
        }
        if let Some(e) = self.pack_prev.bg {
            v.push(e);
        }
        if let Some(e) = self.pack_prev.text {
            v.push(e);
        }
        if let Some(e) = self.pack_next.bg {
            v.push(e);
        }
        if let Some(e) = self.pack_next.text {
            v.push(e);
        }
        if let Some(e) = self.pack_value_bg.entity {
            v.push(e);
        }
        if let Some(e) = self.pack_label {
            v.push(e);
        }
        if let Some(e) = self.pack_value {
            v.push(e);
        }
        if let Some(e) = self.shader_prev.bg {
            v.push(e);
        }
        if let Some(e) = self.shader_prev.text {
            v.push(e);
        }
        if let Some(e) = self.shader_next.bg {
            v.push(e);
        }
        if let Some(e) = self.shader_next.text {
            v.push(e);
        }
        if let Some(e) = self.shader_value_bg.entity {
            v.push(e);
        }
        if let Some(e) = self.shader_label {
            v.push(e);
        }
        if let Some(e) = self.shader_value {
            v.push(e);
        }
        if let Some(e) = self.mods_button.bg {
            v.push(e);
        }
        if let Some(e) = self.mods_button.text {
            v.push(e);
        }
        if self.show_menu_button {
            if let Some(e) = self.menu_button.bg {
                v.push(e);
            }
            if let Some(e) = self.menu_button.text {
                v.push(e);
            }
        }
        v
    }

    // Сброс полей и удаление сущностей (по завершении анимации закрытия).
    fn destroy_content(&mut self, ecs: &mut EcsAdapter) {
        destroy_panel(ecs, &mut self.panel);
        destroy_checkbox(ecs, &mut self.vsync);
        destroy_slider(ecs, &mut self.zoom_speed);
        destroy_checkbox(ecs, &mut self.music);
        destroy_checkbox(ecs, &mut self.sfx);
        destroy_button(ecs, &mut self.pack_prev);
        destroy_button(ecs, &mut self.pack_next);
        destroy_panel(ecs, &mut self.pack_value_bg);
        destroy_button(ecs, &mut self.shader_prev);
        destroy_button(ecs, &mut self.shader_next);
        destroy_panel(ecs, &mut self.shader_value_bg);
        destroy_button(ecs, &mut self.mods_button);
        if self.show_menu_button {
            destroy_button(ecs, &mut self.menu_button);
        }
        if let Some(ent) = self.pack_label.take() {
            ecs.delete_entity(ent);
        }
        if let Some(key) = self.pack_label_key.take() {
            ecs.sprite_cache.remove(&key);
        }
        if let Some(ent) = self.pack_value.take() {
            ecs.delete_entity(ent);
        }
        if let Some(key) = self.pack_value_key.take() {
            ecs.sprite_cache.remove(&key);
        }
        if let Some(ent) = self.shader_label.take() {
            ecs.delete_entity(ent);
        }
        if let Some(key) = self.shader_label_key.take() {
            ecs.sprite_cache.remove(&key);
        }
        if let Some(ent) = self.shader_value.take() {
            ecs.delete_entity(ent);
        }
        if let Some(key) = self.shader_value_key.take() {
            ecs.sprite_cache.remove(&key);
        }
        if let Some(ent) = self.title.take() {
            ecs.delete_entity(ent);
        }
        for cb in [&mut self.vsync, &mut self.music, &mut self.sfx] {
            cb.hover_scale = 1.0;
        }
        self.zoom_speed.hover_scale = 1.0;
        self.vsync_toggled = false;
        self.zoom_speed_changed = false;
        self.music_toggled = false;
        self.sfx_toggled = false;
        self.texture_pack_changed = false;
        self.shader_changed = false;
        self.mods_requested = false;
        self.menu_requested = false;
    }

    /// Открывает окно: создаёт подложку, элементы управления и заголовок.
    pub fn open(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.open {
            return;
        }
        if self.anim.is_active() {
            self.anim.cancel(ecs);
            self.destroy_content(ecs);
        }
        self.open = true;
        create_panel(ecs, device, queue, &mut self.panel);
        create_checkbox(ecs, text_renderer, device, queue, &mut self.vsync);
        create_slider(ecs, text_renderer, device, queue, &mut self.zoom_speed);
        create_checkbox(ecs, text_renderer, device, queue, &mut self.music);
        create_checkbox(ecs, text_renderer, device, queue, &mut self.sfx);
        let saved = self.texture_pack_id();
        self.refresh_pack_list(saved.as_deref());
        create_button(ecs, text_renderer, device, queue, &mut self.pack_prev);
        create_button(ecs, text_renderer, device, queue, &mut self.pack_next);
        create_panel(ecs, device, queue, &mut self.pack_value_bg);
        let (label, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            None,
            None,
            "Texture Pack",
            44.0,
            -2.35,
            Self::PACK_ROW_Y,
            2.8,
            1.2,
            WHITE,
        );
        self.pack_label = label;
        self.pack_label_key = key;
        self.update_pack_value(ecs, text_renderer, device, queue);
        let saved_shader = self.shader_id();
        self.refresh_shader_list(saved_shader.as_deref());
        create_button(ecs, text_renderer, device, queue, &mut self.shader_prev);
        create_button(ecs, text_renderer, device, queue, &mut self.shader_next);
        create_panel(ecs, device, queue, &mut self.shader_value_bg);
        let (label, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            None,
            None,
            "Shader",
            44.0,
            -2.35,
            Self::SHADER_ROW_Y,
            2.8,
            1.2,
            WHITE,
        );
        self.shader_label = label;
        self.shader_label_key = key;
        self.update_shader_value(ecs, text_renderer, device, queue);
        create_button(ecs, text_renderer, device, queue, &mut self.mods_button);
        if self.show_menu_button {
            create_button(ecs, text_renderer, device, queue, &mut self.menu_button);
        }
        let title = text_renderer.add_text(
            ecs, device, queue, "Settings", 64.0, 0.0, 2.4, 4.0, 2.0, WHITE,
        );
        self.title = Some(title);

        let ents = self.collect_entities();
        self.anim.start_open(ecs, ents);
    }

    /// Закрывает окно: запускает анимацию ухода, сущности удаляются в tick().
    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open {
            return;
        }
        self.open = false;
        let ents = self.collect_entities();
        self.anim.start_close(ecs, ents);
    }

    /// Мгновенно убирает окно (используется при смене состояния меню).
    pub fn reset(&mut self, ecs: &mut EcsAdapter) {
        self.anim.cancel(ecs);
        self.open = false;
        self.destroy_content(ecs);
    }

    /// Переносит сохранённые настройки (data/settings.json) в элементы панели.
    /// Вызывается при входе в игру до открытия окна: галочки и ползунок
    /// создаются уже с нужными значениями.
    pub fn apply_saved(&mut self, s: &crate::save::GameSettings) {
        self.vsync.checked = s.vsync;
        self.zoom_speed.value = s.zoom_speed.clamp(self.zoom_speed.min, self.zoom_speed.max);
        self.music.checked = s.music;
        self.sfx.checked = s.sfx;
        self.refresh_pack_list(s.texture_pack.as_deref());
        self.refresh_shader_list(s.shader.as_deref());
    }

    /// Ежекадровая анимация окна: вызывает PanelAnim и подчищает поля.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.anim.tick(ecs, dt) == AnimEvent::Closed {
            self.destroy_content(ecs);
        }
    }

    /// Ежекадровый hover-эффект: элементы плавно увеличиваются при наведении.
    pub fn tick_hover(
        &mut self,
        ecs: &mut EcsAdapter,
        input: &dyn InputSource,
        window_size: (f32, f32),
        dt: f64,
    ) {
        if !self.open {
            return;
        }
        update_checkbox_hover(ecs, &mut self.vsync, input, window_size, dt);
        update_checkbox_hover(ecs, &mut self.music, input, window_size, dt);
        update_checkbox_hover(ecs, &mut self.sfx, input, window_size, dt);
        update_slider_hover(ecs, &mut self.zoom_speed, input, window_size, dt);
        update_button_hover(ecs, &mut self.pack_prev, input, window_size, dt);
        update_button_hover(ecs, &mut self.pack_next, input, window_size, dt);
        update_button_hover(ecs, &mut self.shader_prev, input, window_size, dt);
        update_button_hover(ecs, &mut self.shader_next, input, window_size, dt);
        update_button_hover(ecs, &mut self.mods_button, input, window_size, dt);
        if self.show_menu_button {
            update_button_hover(ecs, &mut self.menu_button, input, window_size, dt);
        }
    }

    /// Обработка кликов по настройкам.
    /// Возвращает true, если клик был обработан (настройки перехватили ввод).
    pub fn handle_input(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &dyn InputSource,
        window_size: (f32, f32),
    ) -> bool {
        if !self.open {
            return false;
        }

        // Ошибка компиляции шейдера появляется после обработки действия сцены,
        // поэтому подпись обновляется на следующем кадре.
        self.update_shader_value(ecs, text_renderer, device, queue);

        // Стрелки выбора текстур-пака: меняем пак без перезагрузки уровня.
        if button_clicked(&self.pack_prev, input, window_size) {
            self.cycle_pack(-1, ecs, text_renderer, device, queue);
            return true;
        }
        if button_clicked(&self.pack_next, input, window_size) {
            self.cycle_pack(1, ecs, text_renderer, device, queue);
            return true;
        }

        // Стрелки выбора шейдера: пересоздаём пайплайны без перезапуска уровня.
        if button_clicked(&self.shader_prev, input, window_size) {
            self.cycle_shader(-1, ecs, text_renderer, device, queue);
            return true;
        }
        if button_clicked(&self.shader_next, input, window_size) {
            self.cycle_shader(1, ecs, text_renderer, device, queue);
            return true;
        }

        // Клик по галочке — переключаем vsync и перерисовываем чекбокс.
        if checkbox_clicked(&self.vsync, input, window_size) {
            self.vsync.checked = !self.vsync.checked;
            refresh_checkbox(ecs, text_renderer, device, queue, &mut self.vsync);
            self.vsync_toggled = true;
            crate::audio::play("click");
            return true;
        }

        // Перетаскивание слайдера — вычисляем значение по позиции курсора.
        if slider_drag(&mut self.zoom_speed, input, window_size) {
            let Some((mx, _)) = input.cursor() else {
                return false;
            };
            let (wx, _) = crate::ui::system::ndc_to_ui(mx, 0.0, window_size);
            let t = ((wx - (self.zoom_speed.x - self.zoom_speed.width / 2.0))
                / self.zoom_speed.width)
                .clamp(0.0, 1.0);
            self.zoom_speed.value =
                self.zoom_speed.min + t * (self.zoom_speed.max - self.zoom_speed.min);
            update_slider_thumb(ecs, device, queue, &mut self.zoom_speed);
            self.zoom_speed_changed = true;
            return true;
        }

        // Чекбокс «Музыка»
        if checkbox_clicked(&self.music, input, window_size) {
            self.music.checked = !self.music.checked;
            refresh_checkbox(ecs, text_renderer, device, queue, &mut self.music);
            self.music_toggled = true;
            crate::audio::play("click");
            return true;
        }

        // Чекбокс «Звуковые эффекты»
        if checkbox_clicked(&self.sfx, input, window_size) {
            self.sfx.checked = !self.sfx.checked;
            refresh_checkbox(ecs, text_renderer, device, queue, &mut self.sfx);
            self.sfx_toggled = true;
            crate::audio::play("click");
            return true;
        }

        // Клик по кнопке «Mods...» — открываем окно модов
        if button_clicked(&self.mods_button, input, window_size) {
            self.mods_requested = true;
            crate::audio::play("click");
            return true;
        }

        // Клик по кнопке «В меню» — запрашиваем выход из игры
        if self.show_menu_button && button_clicked(&self.menu_button, input, window_size) {
            self.menu_requested = true;
            crate::audio::play("click");
            return true;
        }

        false
    }
}
