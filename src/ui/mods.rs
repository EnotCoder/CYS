// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  mods — окно выбора модов: список найденных модов с галочками вкл/выкл,
//  статусом загрузки и подсказкой о перезапуске для логических модов.
// ========================================================================

use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::text_renderer::TextRenderer;
use crate::ui::{
    button_clicked, checkbox_clicked, create_button, create_checkbox, create_panel,
    destroy_button, destroy_checkbox, destroy_panel, refresh_checkbox, update_button_hover,
    update_checkbox_hover, Button, Checkbox, Panel,
};
use crate::EcsAdapter;
use specs::Entity;

const MAX_ROWS: usize = 6;
const ROW_Y_TOP: f32 = 2.0;
const ROW_STEP: f32 = 0.72;
const CHECKBOX_X: f32 = -3.7;
const STATUS_X: f32 = -2.17;

struct ModRow {
    checkbox: Checkbox,
    status: Option<Entity>,
    status_key: Option<u64>,
}

/// Окно модов. Элементы создаются только в открытом состоянии, как в настройках.
pub struct Mods {
    pub open: bool,
    pub panel: Panel,
    title: Option<Entity>,
    rows: Vec<ModRow>,
    mods: Vec<crate::scripts::mods::ModInfo>,
    enabled: Vec<String>,
    hint: Option<Entity>,
    hint_key: Option<u64>,
    close_button: Button,
    anim: PanelAnim,
    /// true, если галочки изменились в этом кадре.
    pub changed: bool,
}

impl Mods {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 10.0, 7.4, 0.9),
            title: None,
            rows: Vec::new(),
            mods: Vec::new(),
            enabled: Vec::new(),
            hint: None,
            hint_key: None,
            close_button: Button::new(0.0, -2.95, 3.0, 0.7, "Close"),
            anim: PanelAnim::new(),
            changed: false,
        }
    }

    /// Текущий выбор галочек — новый порядок включения модов.
    pub fn enabled_ids(&self) -> Vec<String> {
        self.enabled.clone()
    }

    fn row_y(index: usize) -> f32 {
        ROW_Y_TOP - index as f32 * ROW_STEP
    }

    fn status(&self, info: &crate::scripts::mods::ModInfo) -> (String, [u8; 3]) {
        if let Some(error) = crate::scripts::mods::errors().remove(&info.id) {
            let flat = error.replace('\n', " ");
            let short: String = flat.chars().take(34).collect();
            return (format!("Error: {short}"), RED);
        }
        if !info.game_version.is_empty() && info.game_version != env!("CARGO_PKG_VERSION") {
            return (format!("Needs game {}", info.game_version), GOLD);
        }
        if !self.enabled.iter().any(|id| id == &info.id) {
            return ("Disabled".to_string(), GRAY);
        }
        if crate::scripts::mods::loaded_ids()
            .iter()
            .any(|id| id == &info.id)
        {
            ("Loaded".to_string(), GREEN)
        } else {
            ("Pending restart".to_string(), GOLD)
        }
    }

    fn status_text(info: &crate::scripts::mods::ModInfo, status: &str) -> String {
        let mut parts = Vec::new();
        if !info.author.is_empty() {
            parts.push(format!("by {}", info.author));
        }
        if !info.version.is_empty() {
            parts.push(format!("v{}", info.version));
        }
        parts.push(status.to_string());
        parts.join(" · ")
    }

    fn set_status_text(
        &mut self,
        index: usize,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let Some(info) = self.mods.get(index) else {
            return;
        };
        let (status, color) = self.status(info);
        let text = Self::status_text(info, &status);
        let row = &mut self.rows[index];
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            row.status,
            row.status_key,
            &text,
            26.0,
            STATUS_X,
            Self::row_y(index) - 0.26,
            4.6,
            0.8,
            color,
        );
        row.status = entity;
        row.status_key = key;
    }

    fn collect_entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();
        if let Some(entity) = self.panel.entity {
            entities.push(entity);
        }
        if let Some(entity) = self.title {
            entities.push(entity);
        }
        for row in &self.rows {
            if let Some(entity) = row.checkbox.box_entity {
                entities.push(entity);
            }
            if let Some(entity) = row.checkbox.label_entity {
                entities.push(entity);
            }
            if let Some(entity) = row.status {
                entities.push(entity);
            }
        }
        if let Some(entity) = self.hint {
            entities.push(entity);
        }
        if let Some(entity) = self.close_button.bg {
            entities.push(entity);
        }
        if let Some(entity) = self.close_button.text {
            entities.push(entity);
        }
        entities
    }

    fn destroy_content(&mut self, ecs: &mut EcsAdapter) {
        destroy_panel(ecs, &mut self.panel);
        for row in &mut self.rows {
            destroy_checkbox(ecs, &mut row.checkbox);
            if let Some(entity) = row.status.take() {
                ecs.delete_entity(entity);
            }
            if let Some(key) = row.status_key.take() {
                ecs.sprite_cache.remove(&key);
            }
        }
        self.rows.clear();
        if let Some(entity) = self.hint.take() {
            ecs.delete_entity(entity);
        }
        if let Some(key) = self.hint_key.take() {
            ecs.sprite_cache.remove(&key);
        }
        destroy_button(ecs, &mut self.close_button);
        if let Some(entity) = self.title.take() {
            ecs.delete_entity(entity);
        }
        self.changed = false;
    }

    /// Перечитывает моды с диска и строит окно.
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
        crate::scripts::mods::rescan();
        self.mods = crate::scripts::mods::list();
        self.enabled = crate::scripts::mods::enabled_ids();
        create_panel(ecs, device, queue, &mut self.panel);
        self.title = Some(text_renderer.add_text(
            ecs, device, queue, "Mods", 60.0, 0.0, 3.05, 3.0, 1.6, WHITE,
        ));

        let entries: Vec<(String, String)> = self
            .mods
            .iter()
            .take(MAX_ROWS)
            .map(|info| {
                let title = if info.description.is_empty() {
                    info.name.clone()
                } else {
                    let short: String = info.description.chars().take(30).collect();
                    format!("{} — {}", info.name, short)
                };
                (title, info.id.clone())
            })
            .collect();
        for (index, (title, id)) in entries.into_iter().enumerate() {
            let mut checkbox =
                Checkbox::new(CHECKBOX_X, Self::row_y(index), &title, self.enabled.contains(&id));
            checkbox.font_size = 30.0;
            create_checkbox(ecs, text_renderer, device, queue, &mut checkbox);
            self.rows.push(ModRow {
                checkbox,
                status: None,
                status_key: None,
            });
            self.set_status_text(index, ecs, text_renderer, device, queue);
        }

        let extra = self.mods.len().saturating_sub(MAX_ROWS);
        let conflicts = crate::scripts::mods::conflicts().len();
        let conflict_hint = if conflicts > 0 {
            format!(" · {conflicts} conflicts (see console)")
        } else {
            String::new()
        };
        let hint = if extra > 0 {
            format!("…and {extra} more · Logic changes need a restart or world re-entry{conflict_hint}")
        } else {
            format!("Logic changes need a restart or world re-entry{conflict_hint}")
        };
        let (entity, key) = text_renderer.set_text(
            ecs, device, queue, None, None, &hint, 26.0, 0.0, -2.35, 8.8, 0.8, GRAY,
        );
        self.hint = entity;
        self.hint_key = key;
        create_button(ecs, text_renderer, device, queue, &mut self.close_button);

        let entities = self.collect_entities();
        self.anim.start_open(ecs, entities);
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open {
            return;
        }
        self.open = false;
        let entities = self.collect_entities();
        self.anim.start_close(ecs, entities);
    }

    /// Перерисовывает статусы после включения/выключения мода.
    pub fn refresh_statuses(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        for index in 0..self.rows.len() {
            self.set_status_text(index, ecs, text_renderer, device, queue);
        }
    }

    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.anim.tick(ecs, dt) == AnimEvent::Closed {
            self.destroy_content(ecs);
        }
    }

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
        for row in &mut self.rows {
            update_checkbox_hover(ecs, &mut row.checkbox, input, window_size, dt);
        }
        update_button_hover(ecs, &mut self.close_button, input, window_size, dt);
    }

    /// Обработка кликов по галочкам и кнопке закрытия.
    pub fn handle_input(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &dyn InputSource,
        window_size: (f32, f32),
    ) {
        if !self.open {
            return;
        }
        for index in 0..self.rows.len() {
            if checkbox_clicked(&self.rows[index].checkbox, input, window_size) {
                let Some(id) = self.mods.get(index).map(|info| info.id.clone()) else {
                    continue;
                };
                let checked = !self.rows[index].checkbox.checked;
                self.rows[index].checkbox.checked = checked;
                refresh_checkbox(
                    ecs,
                    text_renderer,
                    device,
                    queue,
                    &mut self.rows[index].checkbox,
                );
                self.set_status_text(index, ecs, text_renderer, device, queue);
                if checked {
                    if !self.enabled.iter().any(|current| current == &id) {
                        self.enabled.push(id);
                    }
                } else {
                    self.enabled.retain(|current| current != &id);
                }
                self.changed = true;
                crate::audio::play("click");
                return;
            }
        }
        if button_clicked(&self.close_button, input, window_size) {
            self.close(ecs);
            crate::audio::play("click");
        }
    }
}
