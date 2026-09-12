// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Mini-tutorial: панель «Как играть» поверх новой игры. Показывает 4 шага
//  первой зарплаты — box → food → rack → cassa. Закрывается по «Got it!»,
//  клику вне панели или Escape.
// ========================================================================

use specs::Entity;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::ui::text_renderer::TextRenderer;

// Геометрия панели (мировые координаты UI, центр экрана в (0,0)).
const PANEL_X: f32 = 0.0;
const PANEL_Y: f32 = 0.3;
const PANEL_W: f32 = 9.0;
const PANEL_H: f32 = 5.2;
const TITLE_Y: f32 = 2.0;
// Строки шагов: y каждого шага.
const ROW_YS: [f32; 4] = [0.95, 0.25, -0.45, -1.15];
const ICON_X: f32 = -3.6;
const ICON_SIZE: f32 = 0.75;
const TEXT_X: f32 = -0.7;
const TEXT_W: f32 = 4.5;
const BUTTON_X: f32 = 0.0;
const BUTTON_Y: f32 = -1.9;
const BUTTON_W: f32 = 3.0;
const BUTTON_H: f32 = 0.7;

// Иконка и текст каждого шага.
const STEPS: [(&str, &str); 4] = [
    ("assets/tex/ui/icon_slots/icon_slots_object/regular/box.png",
     "Place a Box to get Food"),
    ("assets/tex/ui/icon_slots/icon_slots_object/regular/box.png",
     "Tap the Box to take the Food"),
    ("assets/tex/ui/icon_slots/icon_slots_object/regular/rack.png",
     "Move the Food to a Rack"),
    ("assets/tex/ui/icon_slots/icon_slots_object/regular/cassa.png",
     "Set a Cassa and start earning Money"),
];

pub struct Tutorial {
    open: bool,
    overlay: Option<Entity>,
    panel: Option<Entity>,
    title: Option<Entity>,
    icons: Vec<Entity>,
    texts: Vec<Entity>,
    button: Option<Entity>,
    button_label: Option<Entity>,
}

impl Tutorial {
    pub fn new() -> Self {
        Self {
            open: false,
            overlay: None,
            panel: None,
            title: None,
            icons: Vec::new(),
            texts: Vec::new(),
            button: None,
            button_label: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Попадание в кнопку «Got it!».
    pub fn button_hit(&self, wx: f32, wy: f32) -> bool {
        (wx - BUTTON_X).abs() <= BUTTON_W / 2.0 && (wy - BUTTON_Y).abs() <= BUTTON_H / 2.0
    }

    /// Попадание внутрь панели.
    pub fn panel_hit(&self, wx: f32, wy: f32) -> bool {
        (wx - PANEL_X).abs() <= PANEL_W / 2.0 && (wy - PANEL_Y).abs() <= PANEL_H / 2.0
    }

    /// Создаёт оверлей обучения поверх игры.
    pub fn open(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.open {
            return;
        }
        self.open = true;

        // Затемнение всего экрана.
        let overlay = ecs.add_ui_sized(0.0, 0.0, 24.0, 16.0, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(overlay, 0.6);
        self.overlay = Some(overlay);

        // Панель в центре.
        let panel = ecs.add_ui_sized(PANEL_X, PANEL_Y, PANEL_W, PANEL_H, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(panel, 0.92);
        self.panel = Some(panel);

        let title = tr.add_text(ecs, device, queue, "HOW TO PLAY", 64.0, PANEL_X, TITLE_Y, 5.0, 2.0, GOLD);
        self.title = Some(title);

        for (i, (icon, text)) in STEPS.iter().enumerate() {
            let y = ROW_YS[i];
            let icon_ent = ecs.add_ui_sized(ICON_X, y, ICON_SIZE, ICON_SIZE, icon, device, queue);
            self.icons.push(icon_ent);
            let text_ent = tr.add_text(ecs, device, queue, text, 40.0, TEXT_X, y, TEXT_W, 1.5, WHITE);
            self.texts.push(text_ent);
        }

        let button = ecs.add_ui_sized(BUTTON_X, BUTTON_Y, BUTTON_W, BUTTON_H, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(button, 0.85);
        self.button = Some(button);
        let label = tr.add_text(ecs, device, queue, "Got it!", 44.0, BUTTON_X, BUTTON_Y, 2.4, 1.5, WHITE);
        self.button_label = Some(label);
    }

    /// Удаляет все сущности панели обучения.
    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open {
            return;
        }
        self.open = false;
        if let Some(e) = self.overlay.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.panel.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.title.take() {
            ecs.delete_entity(e);
        }
        for e in self.icons.drain(..) {
            ecs.delete_entity(e);
        }
        for e in self.texts.drain(..) {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.button.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.button_label.take() {
            ecs.delete_entity(e);
        }
    }
}