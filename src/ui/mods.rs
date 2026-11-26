// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  mods — окно выбора модов: галочки вкл/выкл, статус загрузки, прокрутка
//  и обновление списка с диска без закрытия окна. Используется в меню и в игре.
// ========================================================================

use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::system::{is_inside, ndc_to_ui};
use crate::ui::text_renderer::TextRenderer;
use crate::ui::{
    button_clicked, checkbox_clicked, create_button, create_checkbox, create_panel, destroy_button,
    destroy_checkbox, destroy_panel, refresh_checkbox, update_button_hover, update_checkbox_hover,
    Button, Checkbox, Panel,
};
use crate::EcsAdapter;
use specs::Entity;

const MAX_ROWS: usize = 6;
const ROW_Y_TOP: f32 = 2.0;
const ROW_STEP: f32 = 0.74;
const ROW_BG_OFFSET_Y: f32 = -0.18;
const ROW_BG_W: f32 = 9.0;
const ROW_BG_H: f32 = 0.66;
const ROW_BG_ALPHA: f32 = 0.22;
const ROW_BG_ALT_ALPHA: f32 = 0.32;
const ROW_BG_HOVER_ALPHA: f32 = 0.62;
const CHECKBOX_X: f32 = -4.0;
const STATUS_X: f32 = -2.3;
const STATUS_W: f32 = 4.4;

const SCROLLBAR_X: f32 = 4.35;
const SCROLLBAR_TOP: f32 = 1.95;
const SCROLLBAR_BOT: f32 = -2.15;
const SCROLLBAR_W: f32 = 0.18;
const SCROLLBAR_H: f32 = SCROLLBAR_TOP - SCROLLBAR_BOT;

const HEADER_W: f32 = 9.4;
const HEADER_Y: f32 = 3.2;
const HEADER_H: f32 = 0.9;
const DIVIDER_Y: f32 = 2.68;
const HINT_Y: f32 = -2.42;
const TIP_Y: f32 = -2.74;
const BUTTON_Y: f32 = -3.22;

struct ModRow {
    checkbox: Checkbox,
    status: Option<Entity>,
    status_key: Option<u64>,
    background: Panel,
    base_alpha: f32,
    hovered: bool,
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

fn drop_text(ecs: &mut EcsAdapter, entity: &mut Option<Entity>, key: &mut Option<u64>) {
    if let Some(entity) = entity.take() {
        ecs.delete_entity(entity);
    }
    if let Some(key) = key.take() {
        ecs.sprite_cache.remove(&key);
    }
}

/// Окно модов. Элементы создаются только в открытом состоянии, как в настройках.
pub struct Mods {
    pub open: bool,
    pub panel: Panel,
    header: Panel,
    divider: Panel,
    title: Option<Entity>,
    subtitle: Option<Entity>,
    count: Option<Entity>,
    count_key: Option<u64>,
    rows: Vec<ModRow>,
    mods: Vec<crate::scripts::mods::ModInfo>,
    enabled: Vec<String>,
    empty: Option<Entity>,
    empty_key: Option<u64>,
    hint: Option<Entity>,
    hint_key: Option<u64>,
    tip: Option<Entity>,
    tip_key: Option<u64>,
    close_button: Button,
    refresh_button: Button,
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
        let mut refresh_button = Button::new(-1.35, BUTTON_Y, 2.4, 0.7, "Refresh");
        refresh_button.font_size = 42.0;
        let mut close_button = Button::new(1.35, BUTTON_Y, 2.4, 0.7, "Close");
        close_button.font_size = 42.0;
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 10.0, 7.4, 0.9),
            header: Panel::textured(0.0, HEADER_Y, HEADER_W, HEADER_H, 0.9, TEX_BUTTON),
            divider: Panel::textured(0.0, DIVIDER_Y, HEADER_W, 0.05, 0.9, TEX_BUTTON),
            title: None,
            subtitle: None,
            count: None,
            count_key: None,
            rows: Vec::new(),
            mods: Vec::new(),
            enabled: Vec::new(),
            empty: None,
            empty_key: None,
            hint: None,
            hint_key: None,
            tip: None,
            tip_key: None,
            close_button,
            refresh_button,
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
        let Some(mod_index) = self.mod_index_at(row_index) else {
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
            24.0,
            STATUS_X,
            Self::row_y(row_index) - 0.27,
            STATUS_W,
            0.8,
            color,
        );
        row.status = entity;
        row.status_key = key;
    }

    /// Мод, соответствующий строке. После прокрутки индекс строки не совпадает
    /// с индексом мода — берём его из самой строки, а не из номера строки.
    fn mod_index_at(&self, row_index: usize) -> Option<usize> {
        self.rows.get(row_index).map(|row| row.mod_index)
    }

    fn mod_id_at(&self, row_index: usize) -> Option<String> {
        let mod_index = self.mod_index_at(row_index)?;
        self.mods.get(mod_index).map(|info| info.id.clone())
    }

    fn hovered_row(&self, input: &dyn InputSource, window_size: (f32, f32)) -> Option<usize> {
        let (mx, my) = input.cursor()?;
        let (wx, wy) = ndc_to_ui(mx, my, window_size);
        self.rows.iter().position(|row| {
            let bg = &row.background;
            is_inside(wx, wy, bg.x, bg.y, bg.w / 2.0, bg.h / 2.0)
        })
    }

    fn default_hint(&self) -> (String, [u8; 3]) {
        let conflicts = crate::scripts::mods::conflicts().len();
        if conflicts > 0 {
            (
                format!("{conflicts} mod conflicts — see console for details"),
                GOLD,
            )
        } else {
            (
                "Logic changes apply after a restart or world re-entry".to_string(),
                GRAY,
            )
        }
    }

    fn set_hint_text(
        &mut self,
        text: &str,
        color: [u8; 3],
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.hint,
            self.hint_key,
            text,
            24.0,
            0.0,
            HINT_Y,
            8.8,
            0.8,
            color,
        );
        self.hint = entity;
        self.hint_key = key;
    }

    fn update_hint_for_hover(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &dyn InputSource,
        window_size: (f32, f32),
    ) {
        let hovered = self.hovered_row(input, window_size).and_then(|row| {
            let mod_index = self.mod_index_at(row)?;
            self.mods
                .get(mod_index)
                .map(|info| info.description.clone())
        });
        let (text, color) = match hovered {
            Some(description) if !description.is_empty() => (description, WHITE),
            _ => self.default_hint(),
        };
        self.set_hint_text(&text, color, ecs, text_renderer, device, queue);
    }

    fn update_header_texts(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let total = self.mods.len();
        let enabled = self.enabled.len();
        let conflicts = crate::scripts::mods::conflicts().len();
        let mut text = format!("{total} mods · {enabled} enabled");
        if conflicts > 0 {
            text.push_str(&format!(" · {conflicts} conflicts"));
        }
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.count,
            self.count_key,
            &text,
            26.0,
            3.35,
            HEADER_Y,
            2.5,
            0.8,
            GOLD,
        );
        self.count = entity;
        self.count_key = key;
    }

    fn update_empty(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.mods.is_empty() {
            const TEXT: &str = "No mods found — add a folder with mod.json to assets/mods/";
            let (entity, key) = text_renderer.set_text(
                ecs,
                device,
                queue,
                self.empty,
                self.empty_key,
                TEXT,
                28.0,
                0.0,
                0.0,
                8.4,
                0.8,
                GRAY,
            );
            self.empty = entity;
            self.empty_key = key;
        } else {
            drop_text(ecs, &mut self.empty, &mut self.empty_key);
        }
    }

    fn update_tip_text(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let tip = if max_scroll(self.mods.len()) > 0 {
            "Wheel or drag to scroll · R to refresh"
        } else {
            "R or Refresh rescans the mods folder"
        };
        let (entity, key) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.tip,
            self.tip_key,
            tip,
            22.0,
            0.0,
            TIP_Y,
            8.0,
            0.7,
            GRAY,
        );
        self.tip = entity;
        self.tip_key = key;
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
            destroy_panel(ecs, &mut row.background);
        }
        self.rows.clear();
        for index in 0..MAX_ROWS {
            let mod_index = self.scroll_offset + index;
            let Some(info) = self.mods.get(mod_index).cloned() else {
                break;
            };
            let base_alpha = if index % 2 == 0 {
                ROW_BG_ALPHA
            } else {
                ROW_BG_ALT_ALPHA
            };
            let mut background = Panel::textured(
                0.0,
                Self::row_y(index) + ROW_BG_OFFSET_Y,
                ROW_BG_W,
                ROW_BG_H,
                base_alpha,
                TEX_BUTTON,
            );
            create_panel(ecs, device, queue, &mut background);
            let name: String = info.name.chars().take(24).collect();
            let checked = self.enabled.iter().any(|id| id == &info.id);
            let mut checkbox = Checkbox::new(CHECKBOX_X, Self::row_y(index), &name, checked);
            checkbox.font_size = 30.0;
            create_checkbox(ecs, text_renderer, device, queue, &mut checkbox);
            self.rows.push(ModRow {
                checkbox,
                status: None,
                status_key: None,
                background,
                base_alpha,
                hovered: false,
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
            TEX_BUTTON,
            device,
            queue,
        );
        ecs.update_sprite_alpha(entity, 0.95);
        self.thumb = Some(entity);
    }

    fn sync_scrollbar(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        if max_scroll(self.mods.len()) == 0 {
            if let Some(entity) = self.track.take() {
                ecs.delete_entity(entity);
            }
        } else if self.track.is_none() {
            let track = ecs.add_ui_sized(
                SCROLLBAR_X,
                (SCROLLBAR_TOP + SCROLLBAR_BOT) / 2.0,
                SCROLLBAR_W,
                SCROLLBAR_H,
                TEX_BUTTON,
                device,
                queue,
            );
            ecs.update_sprite_alpha(track, 0.45);
            self.track = Some(track);
        }
        self.update_thumb(ecs, device, queue);
    }

    fn collect_entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();
        for panel in [&self.panel, &self.header, &self.divider] {
            if let Some(entity) = panel.entity {
                entities.push(entity);
            }
        }
        if let Some(entity) = self.title {
            entities.push(entity);
        }
        if let Some(entity) = self.subtitle {
            entities.push(entity);
        }
        if let Some(entity) = self.count {
            entities.push(entity);
        }
        for row in &self.rows {
            if let Some(entity) = row.background.entity {
                entities.push(entity);
            }
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
        for entity in [self.empty, self.hint, self.tip] {
            if let Some(entity) = entity {
                entities.push(entity);
            }
        }
        for button in [&self.refresh_button, &self.close_button] {
            if let Some(entity) = button.bg {
                entities.push(entity);
            }
            if let Some(entity) = button.text {
                entities.push(entity);
            }
        }
        entities
    }

    fn destroy_content(&mut self, ecs: &mut EcsAdapter) {
        destroy_panel(ecs, &mut self.panel);
        destroy_panel(ecs, &mut self.header);
        destroy_panel(ecs, &mut self.divider);
        for row in &mut self.rows {
            destroy_checkbox(ecs, &mut row.checkbox);
            if let Some(entity) = row.status.take() {
                ecs.delete_entity(entity);
            }
            if let Some(key) = row.status_key.take() {
                ecs.sprite_cache.remove(&key);
            }
            destroy_panel(ecs, &mut row.background);
        }
        self.rows.clear();
        for entity in [self.track.take(), self.thumb.take()].into_iter().flatten() {
            ecs.delete_entity(entity);
        }
        if let Some(entity) = self.title.take() {
            ecs.delete_entity(entity);
        }
        if let Some(entity) = self.subtitle.take() {
            ecs.delete_entity(entity);
        }
        drop_text(ecs, &mut self.count, &mut self.count_key);
        drop_text(ecs, &mut self.empty, &mut self.empty_key);
        drop_text(ecs, &mut self.hint, &mut self.hint_key);
        drop_text(ecs, &mut self.tip, &mut self.tip_key);
        destroy_button(ecs, &mut self.refresh_button);
        destroy_button(ecs, &mut self.close_button);
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
        create_panel(ecs, device, queue, &mut self.header);
        create_panel(ecs, device, queue, &mut self.divider);
        self.title = Some(text_renderer.add_text(
            ecs, device, queue, "Mods", 54.0, -3.55, HEADER_Y, 2.0, 1.0, GOLD,
        ));
        self.subtitle = Some(text_renderer.add_text(
            ecs,
            device,
            queue,
            "Player content & Lua scripts",
            22.0,
            -0.2,
            HEADER_Y,
            4.4,
            0.8,
            GRAY,
        ));
        self.update_header_texts(ecs, text_renderer, device, queue);
        self.sync_scrollbar(ecs, device, queue);
        self.rebuild_rows(ecs, text_renderer, device, queue);
        self.update_empty(ecs, text_renderer, device, queue);
        let (hint, color) = self.default_hint();
        self.set_hint_text(&hint, color, ecs, text_renderer, device, queue);
        self.update_tip_text(ecs, text_renderer, device, queue);
        create_button(ecs, text_renderer, device, queue, &mut self.refresh_button);
        create_button(ecs, text_renderer, device, queue, &mut self.close_button);

        let entities = self.collect_entities();
        self.anim.start_open(ecs, entities);
    }

    /// Обновляет список модов с диска, не закрывая окно. Текущие галочки и
    /// порядок включения сохраняются; исчезнувшие моды убираются из списка.
    pub fn refresh(
        &mut self,
        ecs: &mut EcsAdapter,
        text_renderer: &mut TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if !self.open {
            return;
        }
        let previous = self.enabled.clone();
        crate::scripts::mods::rescan();
        self.mods = crate::scripts::mods::list();
        self.enabled = crate::scripts::mods::enabled_ids();
        if self.enabled != previous {
            self.changed = true;
        }
        self.scroll_offset = self.scroll_offset.min(max_scroll(self.mods.len()));
        self.drag_start_y = None;
        self.thumb_dragging = false;
        let animating = self.anim.is_active();
        if animating {
            self.anim.cancel(ecs);
        }
        self.rebuild_rows(ecs, text_renderer, device, queue);
        self.sync_scrollbar(ecs, device, queue);
        self.update_header_texts(ecs, text_renderer, device, queue);
        self.update_empty(ecs, text_renderer, device, queue);
        let (hint, color) = self.default_hint();
        self.set_hint_text(&hint, color, ecs, text_renderer, device, queue);
        self.update_tip_text(ecs, text_renderer, device, queue);
        crate::audio::play("click");
        if animating {
            self.anim.start_open(ecs, self.collect_entities());
        }
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
                let next = (self.thumb_grab_offset + dy / (SCROLLBAR_H * 0.8) * max as f32).round()
                    as usize;
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
        if !self.open || self.anim.is_active() {
            return;
        }
        let hovered = self.hovered_row(input, window_size);
        for (index, row) in self.rows.iter_mut().enumerate() {
            let is_hovered = hovered == Some(index);
            if row.hovered != is_hovered {
                row.hovered = is_hovered;
                if let Some(entity) = row.background.entity {
                    ecs.update_sprite_alpha(
                        entity,
                        if is_hovered {
                            ROW_BG_HOVER_ALPHA
                        } else {
                            row.base_alpha
                        },
                    );
                }
            }
            update_checkbox_hover(ecs, &mut row.checkbox, input, window_size, dt);
        }
        update_button_hover(ecs, &mut self.refresh_button, input, window_size, dt);
        update_button_hover(ecs, &mut self.close_button, input, window_size, dt);
    }

    /// Обработка прокрутки, галочек и кнопок окна.
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
            self.sync_scrollbar(ecs, device, queue);
        }

        for index in 0..self.rows.len() {
            if checkbox_clicked(&self.rows[index].checkbox, input, window_size) {
                let Some(id) = self.mod_id_at(index) else {
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

        self.update_hint_for_hover(ecs, text_renderer, device, queue, input, window_size);

        if button_clicked(&self.refresh_button, input, window_size)
            || input.key_pressed(winit::keyboard::KeyCode::KeyR)
        {
            self.refresh(ecs, text_renderer, device, queue);
            return;
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
        assert!((thumb_y(0, 9) - (SCROLLBAR_TOP - thumb_height(9) / 2.0)).abs() < 0.0001);
        assert!((thumb_y(3, 9) - (SCROLLBAR_BOT + thumb_height(9) / 2.0)).abs() < 0.0001);
    }

    #[test]
    fn row_uses_its_mod_index_after_scrolling() {
        use crate::scripts::mods::ModInfo;

        let mut panel = Mods::new();
        panel.scroll_offset = 3;
        panel.mods = (0..9)
            .map(|index| ModInfo {
                id: format!("mod_{index}"),
                name: format!("Mod {index}"),
                description: String::new(),
                author: String::new(),
                version: String::new(),
                entry: String::new(),
                load_after: Vec::new(),
                game_version: String::new(),
                use_mod: None,
            })
            .collect();
        panel.rows = (0..MAX_ROWS)
            .map(|row| ModRow {
                checkbox: Checkbox::new(CHECKBOX_X, Mods::row_y(row), "Mod", false),
                status: None,
                status_key: None,
                background: Panel::new(0.0, 0.0, ROW_BG_W, ROW_BG_H, ROW_BG_ALPHA),
                base_alpha: ROW_BG_ALPHA,
                hovered: false,
                mod_index: panel.scroll_offset + row,
            })
            .collect();

        assert_eq!(panel.mod_id_at(0).as_deref(), Some("mod_3"));
        assert_eq!(panel.mod_id_at(5).as_deref(), Some("mod_8"));
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
