// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Окно настроек: чекбокс Vertical Sync и слайдер скорости зума.
//  Все элементы создаются только в открытом состоянии и уничтожаются при
//  закрытии, чтобы не занимать сущности/спрайты в ECS.
// ========================================================================

use specs::Entity;
use crate::ui::{Panel, Checkbox, Slider, Button, create_panel, destroy_panel, create_checkbox, destroy_checkbox, refresh_checkbox, checkbox_clicked, checkbox_hovered, create_slider, destroy_slider, slider_drag, slider_hovered, update_slider_thumb, create_button, destroy_button, button_clicked};
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;

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
    /// Текущий масштаб галочки и ползунка (hover-анимация, ease к 1.0)
    checkbox_scale: f32,
    slider_scale: f32,
    /// Кнопка «В меню» (выход из игры в выбор миров)
    menu_button: Button,
    /// Флаг запроса выхода в меню, устанавливается по клику на menu_button
    pub menu_requested: bool,
    anim: PanelAnim,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 9.0, 6.0, 0.85),
            title: None,
            vsync: Checkbox::new(-1.4, 0.3, "Vertical Sync", true),
            vsync_toggled: false,
            zoom_speed: Slider::new(-0.1, -0.4, "Zoom Speed", 0.02, 0.3, 0.1),
            zoom_speed_changed: false,
            music: Checkbox::new(-1.4, -1.1, "Music", true),
            sfx: Checkbox::new(-1.4, -1.8, "Sound Effects", true),
            music_toggled: false,
            sfx_toggled: false,
            checkbox_scale: 1.0,
            slider_scale: 1.0,
            menu_button: Button::new(0.0, -2.55, 3.0, 0.7, "В меню"),
            menu_requested: false,
            anim: PanelAnim::new(),
        }
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
        if let Some(e) = self.menu_button.bg {
            v.push(e);
        }
        if let Some(e) = self.menu_button.text {
            v.push(e);
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
        destroy_button(ecs, &mut self.menu_button);
        if let Some(ent) = self.title.take() {
            ecs.delete_entity(ent);
        }
        self.checkbox_scale = 1.0;
        self.slider_scale = 1.0;
        self.music_toggled = false;
        self.sfx_toggled = false;
        self.menu_requested = false;
    }

    /// Открывает окно: создаёт подложку, элементы управления и заголовок.
    pub fn open(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.open { return; }
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
        create_button(ecs, text_renderer, device, queue, &mut self.menu_button);
        let title = text_renderer.add_text(ecs, device, queue, "Settings", 64.0, 0.0, 1.8, 4.0, 2.0, WHITE);
        self.title = Some(title);

        let ents = self.collect_entities();
        self.anim.start_open(ecs, ents);
    }

    /// Закрывает окно: запускает анимацию ухода, сущности удаляются в tick().
    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        let ents = self.collect_entities();
        self.anim.start_close(ecs, ents);
    }

    /// Переносит сохранённые настройки (settings.json) в элементы панели.
    /// Вызывается при входе в игру до открытия окна: галочки и ползунок
    /// создаются уже с нужными значениями.
    pub fn apply_saved(&mut self, s: &crate::save::GameSettings) {
        self.vsync.checked = s.vsync;
        self.zoom_speed.value = s.zoom_speed.clamp(self.zoom_speed.min, self.zoom_speed.max);
        self.music.checked = s.music;
        self.sfx.checked = s.sfx;
    }

    /// Ежекадровая анимация окна: вызывает PanelAnim и подчищает поля.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.anim.tick(ecs, dt) == AnimEvent::Closed {
            self.destroy_content(ecs);
        }
    }

    /// Ежекадровый hover-эффект: галочка и ползунок плавно увеличиваются
    /// при наведении и возвращаются к обычному размеру вне его.
    pub fn tick_hover(&mut self, ecs: &mut EcsAdapter, input: &dyn InputSource, window_size: (f32, f32), dt: f64) {
        if !self.open {
            return;
        }
        // Цели: 1.15 при наведении, 1.0 иначе.
        let cb_hover = checkbox_hovered(&self.vsync, input, window_size)
            || checkbox_hovered(&self.music, input, window_size)
            || checkbox_hovered(&self.sfx, input, window_size);
        let sl_hover = slider_hovered(&self.zoom_speed, input, window_size);
        let cb_target = if cb_hover { 1.15 } else { 1.0 };
        let sl_target = if sl_hover { 1.15 } else { 1.0 };
        let k = (12.0 * dt as f32).min(1.0);
        self.checkbox_scale += (cb_target - self.checkbox_scale) * k;
        self.slider_scale += (sl_target - self.slider_scale) * k;
        if (cb_target - self.checkbox_scale).abs() < 0.0001 { self.checkbox_scale = cb_target; }
        if (sl_target - self.slider_scale).abs() < 0.0001 { self.slider_scale = sl_target; }
        for ent in [self.vsync.box_entity, self.music.box_entity, self.sfx.box_entity].into_iter().flatten() {
            ecs.update_sprite_scale(ent, self.checkbox_scale);
        }
        if let Some(e) = self.zoom_speed.thumb {
            ecs.update_sprite_scale(e, self.slider_scale);
        }
    }

    /// Обработка кликов по настройкам.
    /// Возвращает true, если клик был обработан (настройки перехватили ввод).
    pub fn handle_input(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, input: &dyn InputSource, window_size: (f32, f32)) -> bool {
        if !self.open { return false; }

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
            let Some((mx, _)) = input.cursor() else { return false };
            let (wx, _) = crate::ui::system::ndc_to_ui(mx, 0.0, window_size);
            let t = ((wx - (self.zoom_speed.x - self.zoom_speed.width / 2.0)) / self.zoom_speed.width).clamp(0.0, 1.0);
            self.zoom_speed.value = self.zoom_speed.min + t * (self.zoom_speed.max - self.zoom_speed.min);
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

        // Клик по кнопке «В меню» — запрашиваем выход из игры
        if button_clicked(&self.menu_button, input, window_size) {
            self.menu_requested = true;
            crate::audio::play("click");
            return true;
        }

        false
    }
}
