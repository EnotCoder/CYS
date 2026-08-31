// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use crate::ui::{Panel, destroy_panel};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;
use crate::ui::system::is_clicked;

const ROW_START_Y: f32 = 1.4;
const ROW_STEP: f32 = -1.1;
const ROW_HALF_H: f32 = 0.5;
const COL_ICON_X: [f32; 2] = [-3.2, 1.8];
const COL_LABEL_X: [f32; 2] = [-2.0, 3.0];
const COL_HALF_W: f32 = 2.2;

const SCROLLBAR_X: f32 = 4.0;
const SCROLLBAR_W: f32 = 0.3;
const SCROLLBAR_TOP: f32 = 2.5;
const SCROLLBAR_BOT: f32 = -2.5;
const SCROLLBAR_H: f32 = 5.0;

struct ShopRow {
    icon: Entity,
    label: Entity,
    label_key: Option<u64>,
    name: String,
    item_idx: usize,
    x: f32,
    y: f32,
}

pub struct Shop {
    pub open: bool,
    pub panel: Panel,
    title: Option<Entity>,
    rows: Vec<ShopRow>,
    pub scroll_offset: f32,
    touch_scroll_start: Option<(f32, f32)>,
    touch_scroll_base: f32,
    track: Option<Entity>,
    thumb: Option<Entity>,
    pub thumb_dragging: bool,
    thumb_grab_y: f32,
    thumb_grab_offset: f32,
}

pub type ShopItem = (String, String, i32, bool);

fn max_scroll(total: usize) -> f32 {
    if total <= 8 { 0.0 } else { (total - 8) as f32 }
}

fn clear_rows(ecs: &mut EcsAdapter, rows: &mut Vec<ShopRow>) {
    for r in rows.drain(..) {
        ecs.delete_entity(r.icon);
        ecs.delete_entity(r.label);
    }
}

fn build_rows(ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem], scroll_offset: f32, rows: &mut Vec<ShopRow>) {
    clear_rows(ecs, rows);
    let top = scroll_offset as usize;
    for row in 0..4 {
        for col in 0..2 {
            let idx = top + row * 2 + col;
            if idx >= items.len() { continue; }
            let (name, icon, price, owned) = &items[idx];
            let y = ROW_START_Y + ROW_STEP * row as f32;
            let x = COL_ICON_X[col];
            let icon_ent = ecs.add_ui_sized(x, y, 0.8, 0.8, icon, device, queue);
            let text = if *owned { "— Owned".to_string() } else { format!("— ${}", price) };
            let label = tr.add_text(ecs, device, queue, &text, 36.0, COL_LABEL_X[col], y, 4.0, 1.0, WHITE);
            rows.push(ShopRow { icon: icon_ent, label, label_key: None, name: name.clone(), item_idx: idx, x: COL_LABEL_X[col], y });
        }
    }
}

fn thumb_y(scroll_offset: f32, total: usize) -> f32 {
    let max = max_scroll(total);
    if max <= 0.0 { return (SCROLLBAR_TOP + SCROLLBAR_BOT) / 2.0; }
    let h = thumb_height(total);
    let pad = h / 2.0;
    let t = scroll_offset / max;
    (SCROLLBAR_TOP - pad) - t * (SCROLLBAR_H - h)
}

fn thumb_height(total: usize) -> f32 {
    if total <= 8 { SCROLLBAR_H } else { (8.0 / total as f32 * SCROLLBAR_H).max(0.4) }
}

fn update_thumb(ecs: &mut EcsAdapter, thumb: &mut Option<Entity>, scroll_offset: f32, total: usize, device: &wgpu::Device, queue: &wgpu::Queue) {
    if total <= 8 {
        if let Some(ent) = thumb.take() { ecs.delete_entity(ent); }
        return;
    }
    if let Some(ent) = thumb.take() { ecs.delete_entity(ent); }
    let y = thumb_y(scroll_offset, total);
    let h = thumb_height(total);
    let ent = ecs.add_ui_sized(SCROLLBAR_X, y, SCROLLBAR_W, h, "assets/tex/dev_tools/black.png", device, queue);
    ecs.update_sprite_alpha(ent, 0.6);
    *thumb = Some(ent);
}

fn scroll_by_input(input: &dyn InputSource, offset: &mut f32, items_len: usize, touch_start: &mut Option<(f32, f32)>, touch_base: &mut f32, panel: &Panel, window_size: (f32, f32)) {
    if items_len == 0 { return; }
    let max = max_scroll(items_len);

    let (_, wy) = input.scroll_diff();
    if wy.abs() > 0.01 {
        let raw = *offset - wy * 4.0;
        *offset = if wy > 0.0 { (raw / 8.0).floor() * 8.0 } else { (raw / 8.0).ceil() * 8.0 };
        *offset = (*offset).clamp(0.0, max);
        *touch_start = None;
        return;
    }

    if let Some((mx, my)) = input.cursor() {
        let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
        let in_panel = wx.abs() < panel.w / 2.0 && wy.abs() < panel.h / 2.0;
        let pressing = input.mouse_held(winit::event::MouseButton::Left);
        if pressing && in_panel {
            match *touch_start {
                Some((_, start_y)) => {
                    let dy = wy - start_y;
                    let raw = *touch_base + dy / (-ROW_STEP) * 2.0;
                    *offset = ((raw / 8.0).floor() * 8.0).clamp(0.0, max);
                }
                None => {
                    *touch_start = Some((wx, wy));
                    *touch_base = *offset;
                }
            }
        } else {
            *touch_start = None;
        }
    } else {
        *touch_start = None;
    }
}

fn scrollbar_input(input: &dyn InputSource, offset: &mut f32, items_len: usize, thumb_dragging: &mut bool, thumb_grab_y: &mut f32, thumb_grab_offset: &mut f32, window_size: (f32, f32)) {
    if items_len <= 8 { return; }
    let max = max_scroll(items_len);

    if let Some((mx, my)) = input.cursor() {
        let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
        let on_track = (wx - SCROLLBAR_X).abs() < SCROLLBAR_W * 2.0 && wy >= SCROLLBAR_BOT && wy <= SCROLLBAR_TOP;
        let pressing = input.mouse_held(winit::event::MouseButton::Left);

        if pressing && (on_track || *thumb_dragging) {
            if !*thumb_dragging {
                *thumb_dragging = true;
                *thumb_grab_y = wy;
                *thumb_grab_offset = *offset;
            }
            let dy = *thumb_grab_y - wy;
            *offset = (*thumb_grab_offset + dy / SCROLLBAR_H * max).clamp(0.0, max);
        } else {
            *thumb_dragging = false;
        }
    } else {
        *thumb_dragging = false;
    }
}

impl Shop {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 9.0, 6.0, 0.85),
            title: None,
            rows: Vec::new(),
            scroll_offset: 0.0,
            touch_scroll_start: None,
            touch_scroll_base: 0.0,
            track: None,
            thumb: None,
            thumb_dragging: false,
            thumb_grab_y: 0.0,
            thumb_grab_offset: 0.0,
        }
    }

    pub fn open(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        if self.open { return; }
        self.open = true;
        self.scroll_offset = 0.0;
        self.touch_scroll_start = None;
        self.thumb_dragging = false;
        self.thumb_grab_y = 0.0;
        self.thumb_grab_offset = 0.0;
        let ent = ecs.add_ui_sized(self.panel.x, self.panel.y, self.panel.w, self.panel.h, "assets/tex/ui/shop_panel.png", device, queue);
        ecs.update_sprite_alpha(ent, self.panel.alpha);
        self.panel.entity = Some(ent);
        self.title = Some(tr.add_text(ecs, device, queue, "Shop", 64.0, 0.0, 2.4, 4.0, 2.0, WHITE));

        if items.len() > 8 {
            let t = ecs.add_ui_sized(SCROLLBAR_X, 0.0, SCROLLBAR_W, SCROLLBAR_H, "assets/tex/dev_tools/black.png", device, queue);
            ecs.update_sprite_alpha(t, 0.3);
            self.track = Some(t);
            let h = thumb_height(items.len());
            let th = ecs.add_ui_sized(SCROLLBAR_X, thumb_y(0.0, items.len()), SCROLLBAR_W, h, "assets/tex/dev_tools/black.png", device, queue);
            ecs.update_sprite_alpha(th, 0.6);
            self.thumb = Some(th);
        }

        build_rows(ecs, tr, device, queue, items, self.scroll_offset, &mut self.rows);
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        destroy_panel(ecs, &mut self.panel);
        clear_rows(ecs, &mut self.rows);
        if let Some(ent) = self.title.take() { ecs.delete_entity(ent); }
        if let Some(ent) = self.track.take() { ecs.delete_entity(ent); }
        if let Some(ent) = self.thumb.take() { ecs.delete_entity(ent); }
    }

    pub fn refresh(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        build_rows(ecs, tr, device, queue, items, self.scroll_offset, &mut self.rows);
        update_thumb(ecs, &mut self.thumb, self.scroll_offset, items.len(), device, queue);
    }

    pub fn scroll_input(&mut self, input: &dyn InputSource, items_len: usize, window_size: (f32, f32)) {
        if !self.open { return; }
        scrollbar_input(input, &mut self.scroll_offset, items_len, &mut self.thumb_dragging, &mut self.thumb_grab_y, &mut self.thumb_grab_offset, window_size);
        if !self.thumb_dragging {
            scroll_by_input(input, &mut self.scroll_offset, items_len, &mut self.touch_scroll_start, &mut self.touch_scroll_base, &self.panel, window_size);
        }
    }

    pub fn row_clicked(&self, input: &dyn InputSource, window_size: (f32, f32)) -> Option<usize> {
        if !self.open { return None; }
        for r in self.rows.iter() {
            if is_clicked(input, window_size, r.x, r.y, COL_HALF_W, ROW_HALF_H) {
                return Some(r.item_idx);
            }
        }
        None
    }
}
