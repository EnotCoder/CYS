// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Окно выбора погоды/сезона: 4 кнопки с иконками (Spring, Summer,
//  Autumn, Winter). Выбор сезона меняет текстуру травы на карте.
// ========================================================================

use specs::Entity;
use crate::ecs::components::Season;
use crate::ui::{Panel, create_panel, destroy_panel};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;

const SEASON_ICON_SIZE: f32 = 0.8;
const SEASON_BTN_START_Y: f32 = 0.8;
const SEASON_BTN_STEP: f32 = -1.1;
const SEASON_COL_X: f32 = -1.2;
const SEASON_LABEL_X: f32 = 0.5;

struct SeasonButton {
    icon: Entity,
    label: Entity,
    x: f32,
    y: f32,
}

pub struct Weather {
    pub open: bool,
    pub panel: Panel,
    pub title: Option<Entity>,
    buttons: Vec<SeasonButton>,
    pub selected: Season,
    pub season_changed: bool,
}

const SEASON_ICONS: [(&str, &str); 4] = [
    ("Spring", "assets/tex/ui/time_of_year/spring.png"),
    ("Summer", "assets/tex/ui/time_of_year/summer.png"),
    ("Autumn", "assets/tex/ui/time_of_year/autumn.png"),
    ("Winter", "assets/tex/ui/time_of_year/winter.png"),
];

impl Weather {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 5.0, 5.5, 0.85),
            title: None,
            buttons: Vec::new(),
            selected: Season::Summer,
            season_changed: false,
        }
    }

    pub fn open(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.open { return; }
        self.open = true;
        create_panel(ecs, device, queue, &mut self.panel);
        self.title = Some(text_renderer.add_text(ecs, device, queue, "Weather", 56.0, 0.0, 2.2, 4.0, 2.0, WHITE));

        let seasons = Season::all();
        for (i, (name, icon_path)) in SEASON_ICONS.iter().enumerate() {
            let y = SEASON_BTN_START_Y + SEASON_BTN_STEP * i as f32;
            let icon = ecs.add_ui_sized(SEASON_COL_X, y, SEASON_ICON_SIZE, SEASON_ICON_SIZE, icon_path, device, queue);
            let alpha = if seasons[i] == self.selected { 1.0 } else { 0.4 };
            ecs.update_sprite_alpha(icon, alpha);
            let label = text_renderer.add_text(ecs, device, queue, name, 42.0, SEASON_LABEL_X, y, 2.5, 1.0, WHITE);
            self.buttons.push(SeasonButton { icon, label, x: SEASON_COL_X, y });
        }
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        destroy_panel(ecs, &mut self.panel);
        if let Some(ent) = self.title.take() { ecs.delete_entity(ent); }
        for btn in self.buttons.drain(..) {
            ecs.delete_entity(btn.icon);
            ecs.delete_entity(btn.label);
        }
    }

    pub fn handle_input(&mut self, ecs: &mut EcsAdapter, _text_renderer: &mut TextRenderer, _device: &wgpu::Device, _queue: &wgpu::Queue, input: &dyn InputSource, window_size: (f32, f32)) {
        if !self.open { return; }
        let seasons = Season::all();
        for (i, btn) in self.buttons.iter().enumerate() {
            if crate::ui::system::is_clicked(input, window_size, btn.x, btn.y, SEASON_ICON_SIZE, SEASON_ICON_SIZE) {
                let new_season = seasons[i];
                if new_season != self.selected {
                    self.selected = new_season;
                    self.season_changed = true;
                    self.update_highlight(ecs);
                    crate::audio::play("click");
                }
                return;
            }
        }
    }

    fn update_highlight(&self, ecs: &mut EcsAdapter) {
        let seasons = Season::all();
        for (i, btn) in self.buttons.iter().enumerate() {
            let alpha = if seasons[i] == self.selected { 1.0 } else { 0.4 };
            ecs.update_sprite_alpha(btn.icon, alpha);
        }
    }
}
