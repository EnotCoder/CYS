// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  mods — окно выбора модов: галочки вкл/выкл, статус загрузки и прокрутка,
//  когда модов больше MAX_ROWS. Используется и в меню, и в игре.
// ========================================================================

use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::system::ndc_to_ui;
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

const SCROLLBAR_X: f32 = 4.3;
const SCROLLBAR_TOP: f32 = 2.0;
const SCROLLBAR_BOT: f32 = -1.8;
const SCROLLBAR_W: f32 = 0.18;
const SCROLLBAR_H: f32 = SCROLLBAR_TOP - SCROLLBAR_BOT;

struct ModRow {
    checkbox: Checkbox,
    status: Option<Entity>,
    status_key: Option<u64>,
    mod_index: usize,
}

pub(crate) fn max_scroll(total: usize) -> usize {
    total.saturating_sub(MAX_ROWS)
}

fn thumb_height(total: usize) -> f32 {
    if total <= MAX_ROWS {
        SCROLLBAR_H
    } else {
        (MAX_ROWS as f32 / total as f32 * SCROLLBAR_H).max(0.4)
    }
}

fn thumb_y(offset: usize, total: usize) -> f32 {
    let max = max_scroll(total);
    if max == 0 {
        return (SCROLLBAR_TOP + SCROLLBAR_BOT) / 2.0;
    }
    let h = thumb_height(total);
    let t = offset as f32 / max as f32;
    (SCROLLBAR_TOP - h / 2.0) - t * (SCROLLBAR_H - h)
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
    scroll_offset: usize,
    scroll_dirty: bool,
    drag_start_y: Option<f32>,
    drag_base: f32,
    thumb_dragging: bool,
    thumb_grab_y: f32,
    thumb_grab_offset: f32,
    track: Option<Entity>,
    thumb: Option<Entity>,
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
            scroll_offset: 0,
            scroll_dirty: false,
            drag_start_y: None,
            drag_base: 0.0,
            thumb_dragging: false,
            thumb_grab_y: 0.0,
            thumb_grab_offset: 0.0,
            track: None,
            thumb: None,
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
        row_index: usize,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let Some(mod_index) = self.rows.get(row_index).map(|row| row.mod_index) else {
            return;
        };
        let Some(info) = self.mods.get(mod_index).cloned() else {
            return;
        };
        let (status, color) = self.status(&info);
        let text = Self::status_text(&info, &status);
        let row = &mut self.rows[row_index];
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            row.status,
            row.status_key,
            &text,
            26.0,
            STATUS_X,
            Self::row_y(row_index) - 0.26,
            4.6,
            0.8,
            color,
        );
        row.status = entity;
        row.status_key = key;
    }

    fn rebuild_rows(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
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
        for index in 0..MAX_ROWS {
            let mod_index = self.scroll_offset + index;
            let Some(info) = self.mods.get(mod_index).cloned() else {
                break;
            };
            let title = if info.description.is_empty() {
                info.name.clone()
            } else {
                let short: String = info.description.chars().take(30).collect();
                format!("{} — {}", info.name, short)
            };
            let checked = self.enabled.iter().any(|id| id == &info.id);
            let mut checkbox = Checkbox::new(CHECKBOX_X, Self::row_y(index), &title, checked);
            checkbox.font_size = 30.0;
            create_checkbox(ecs, text_renderer, device, queue, &mut checkbox);
            self.rows.push(ModRow {
                checkbox,
                status: None,
                status_key: None,
                mod_index,
            });
            self.set_status_text(index, ecs, text_renderer, device, queue);
        }
        self.scroll_dirty = false;
    }

    fn update_thumb(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Some(entity) = self.thumb.take() {
            ecs.delete_entity(entity);
        }
        if max_scroll(self.mods.len()) == 0 {
            return;
        }
        let entity = ecs.add_ui_sized(
            SCROLLBAR_X,
            thumb_y(self.scroll_offset, self.mods.len()),
            SCROLLBAR_W * 1.8,
            thumb_height(self.mods.len()),
            "assets/tex/dev_tools/black.png",
            device,
            queue,
        );
        ecs.update_sprite_alpha(entity, 0.8);
        self.thumb = Some(entity);
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
        if let Some(entity) = self.track {
            entities.push(entity);
        }
        if let Some(entity) = self.thumb {
            entities.push(entity);
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
        for entity in [self.track.take(), self.thumb.take()].into_iter().flatten() {
            ecs.delete_entity(entity);
        }
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
        self.scroll_dirty = false;
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
        self.scroll_offset = 0;
        self.drag_start_y = None;
        self.thumb_dragging = false;
        create_panel(ecs, device, queue, &mut self.panel);
        self.title = Some(text_renderer.add_text(
            ecs, device, queue, "Mods", 60.0, 0.0, 3.05, 3.0, 1.6, WHITE,
        ));

        if max_scroll(self.mods.len()) > 0 {
            let track = ecs.add_ui_sized(
                SCROLLBAR_X,
                (SCROLLBAR_TOP + SCROLLBAR_BOT) / 2.0,
                SCROLLBAR_W,
                SCROLLBAR_H,
                "assets/tex/dev_tools/black.png",
                device,
                queue,
            );
            ecs.update_sprite_alpha(track, 0.35);
            self.track = Some(track);
        }
        self.rebuild_rows(ecs, text_renderer, device, queue);
        self.update_thumb(ecs, device, queue);

        let scroll_hint = if max_scroll(self.mods.len()) > 0 {
            " · scroll to see more"
        } else {
            ""
        };
        let conflicts = crate::scripts::mods::conflicts().len();
        let conflict_hint = if conflicts > 0 {
            format!(" · {conflicts} conflicts (see console)")
        } else {
            String::new()
        };
        let hint = format!(
            "Logic changes need a restart or world re-entry{scroll_hint}{conflict_hint}"
        );
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

    /// Мгновенно убирает окно (используется при выходе из меню).
    pub fn reset(&mut self, ecs: &mut EcsAdapter) {
        self.anim.cancel(ecs);
        self.open = false;
        self.destroy_content(ecs);
    }

    fn handle_scroll(&mut self, input: &dyn InputSource, window_size: (f32, f32)) {
        let max = max_scroll(self.mods.len());
        if max == 0 {
            return;
        }
        let (_, wy) = input.scroll_diff();
        if wy.abs() > 0.01 {
            let raw = self.scroll_offset as f32 - wy;
            let next = (raw.round() as usize).clamp(0, max);
            if next != self.scroll_offset {
                self.scroll_offset = next;
                self.scroll_dirty = true;
            }
            self.drag_start_y = None;
            return;
        }
        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = ndc_to_ui(mx, my, window_size);
            let inside = (wx - self.panel.x).abs() <= self.panel.w / 2.0
                && (wy - self.panel.y).abs() <= self.panel.h / 2.0;
            let pressing = input.mouse_held(winit::event::MouseButton::Left);
            if pressing && inside && !self.thumb_dragging {
                match self.drag_start_y {
                    Some(start) => {
                        let raw = self.drag_base + (wy - start) / ROW_STEP;
                        let next = (raw.round() as usize).clamp(0, max);
                        if next != self.scroll_offset {
                            self.scroll_offset = next;
                            self.scroll_dirty = true;
                        }
                    }
                    None => {
                        self.drag_start_y = Some(wy);
                        self.drag_base = self.scroll_offset as f32;
                    }
                }
            } else {
                self.drag_start_y = None;
            }
        } else {
            self.drag_start_y = None;
        }
    }

    fn scrollbar_input(&mut self, input: &dyn InputSource, window_size: (f32, f32)) {
        let max = max_scroll(self.mods.len());
        if max == 0 {
            return;
        }
        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = ndc_to_ui(mx, my, window_size);
            let on_track = (wx - SCROLLBAR_X).abs() < SCROLLBAR_W * 4.0
                && wy >= SCROLLBAR_BOT - 0.5
                && wy <= SCROLLBAR_TOP + 0.5;
            let pressing = input.mouse_held(winit::event::MouseButton::Left);
            if pressing && (on_track || self.thumb_dragging) {
                if !self.thumb_dragging {
                    self.thumb_dragging = true;
                    self.thumb_grab_y = wy;
                    self.thumb_grab_offset = self.scroll_offset as f32;
                }
                let dy = self.thumb_grab_y - wy;
                let next = (self.thumb_grab_offset + dy / (SCROLLBAR_H * 0.8) * max as f32)
                    .round() as usize;
                let next = next.clamp(0, max);
                if next != self.scroll_offset {
                    self.scroll_offset = next;
                    self.scroll_dirty = true;
                }
            } else {
                self.thumb_dragging = false;
            }
        } else {
            self.thumb_dragging = false;
        }
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

    /// Обработка прокрутки, галочек и кнопки закрытия.
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
        self.handle_scroll(input, window_size);
        self.scrollbar_input(input, window_size);
        if self.scroll_dirty {
            self.rebuild_rows(ecs, text_renderer, device, queue);
            self.update_thumb(ecs, device, queue);
        }

        for index in 0..self.rows.len() {
            if checkbox_clicked(&self.rows[index].checkbox, input, window_size) {
                let Some(id) = self.mods.get(index).map(|_| self.rows[index].mod_index).and_then(
                    |mod_index| self.mods.get(mod_index).map(|info| info.id.clone()),
                ) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_math_clamps_to_available_rows() {
        assert_eq!(max_scroll(0), 0);
        assert_eq!(max_scroll(MAX_ROWS), 0);
        assert_eq!(max_scroll(MAX_ROWS + 3), 3);
        assert_eq!(thumb_y(0, 9), SCROLLBAR_TOP - thumb_height(9) / 2.0);
        assert_eq!(thumb_y(3, 9), SCROLLBAR_BOT + thumb_height(9) / 2.0);
    }

    #[test]
    fn shared_status_sprite_survives_switching_the_other_mod() {
        use crate::ecs::components::SpriteComponent;
        use specs::{Builder, WorldExt};
        use std::sync::Arc;

        let mut ecs = crate::EcsAdapter::new();
        let status_text = "by CYS · v1.0.0 · Disabled";
        let text_path = "__text__by CYS · v1.0.0 · Disabled@26_ol0.8_cc8c8c8";
        let shared_key = TextRenderer::sprite_cache_key(status_text, 26.0, 0.8, GRAY);

        let mut add_status = || {
            ecs.world
                .create_entity()
                .with(SpriteComponent {
                    texture_path: Arc::from(text_path),
                    texture_frame: [0, 0],
                    texture_count: [1, 1],
                    scale: 1.0,
                    alpha: 1.0,
                    animated: false,
                    frame_paths: Vec::new(),
                    current_frame: 0,
                    anim_fps: 1.0,
                    anim_loop: true,
                    anim_timer: 0.0,
                })
                .build()
        };

        let disabled_row = add_status();
        let other_row = add_status();

        // После выключения одного мода и включения другого общий спрайт
        // статуса ещё нужен выключенной строке и не должен удаляться из кэша.
        assert!(!ecs.remove_sprite_if_unused(shared_key, text_path, Some(disabled_row)));
        assert!(ecs.texture_path_in_use(text_path, Some(other_row)));

        ecs.delete_entity(other_row);
        assert!(ecs.remove_sprite_if_unused(shared_key, text_path, Some(disabled_row)));
    }
}
