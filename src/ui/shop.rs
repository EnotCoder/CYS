// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use crate::ui::{Panel, destroy_panel};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;
use crate::ui::system::is_clicked;

/// Раскладка строк каталога в окне магазина (2 столбца × 4 строки).
const ROW_START_Y: f32 = 1.4;
const ROW_STEP: f32 = -1.1;
const ROW_HALF_H: f32 = 0.5;
const ROW_HALF_W: f32 = 2.0;

/// Столбцы: левый и правый.
const COL_ICON_X: [f32; 2] = [-3.2, 1.8];
const COL_LABEL_X: [f32; 2] = [-2.0, 3.0];
const COL_HALF_W: f32 = 2.2;

/// Одна строка каталога: иконка предмета, подпись и имя объекта.
struct ShopRow {
    icon: Entity,
    label: Entity,
    label_key: Option<u64>,
    name: String,
    item_idx: usize,
    x: f32,
    y: f32,
}

/// Состояние окна магазина и его UI-элементы.
pub struct Shop {
    pub open: bool,
    pub panel: Panel,
    title: Option<Entity>,
    rows: Vec<ShopRow>,
    pub scroll_offset: f32,
    touch_scroll_start: Option<(f32, f32)>,
    touch_scroll_base: f32,
}

pub type ShopItem = (String, String, i32, bool);

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
        }
    }

    fn max_scroll(total: usize) -> f32 {
        if total <= 8 { 0.0 } else { ((total - 8) as f32).max(0.0) }
    }

    pub fn open(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        if self.open { return; }
        self.open = true;
        self.scroll_offset = 0.0;
        self.touch_scroll_start = None;
        let ent = ecs.add_ui_sized(self.panel.x, self.panel.y, self.panel.w, self.panel.h, "assets/tex/ui/shop_panel.png", device, queue);
        ecs.update_sprite_alpha(ent, self.panel.alpha);
        self.panel.entity = Some(ent);
        self.title = Some(tr.add_text(ecs, device, queue, "Shop", 64.0, 0.0, 2.4, 4.0, 2.0, WHITE));
        self.build_rows(ecs, tr, device, queue, items);
    }

    fn build_rows(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        self.clear_rows(ecs);
        let top = self.scroll_offset as usize;
        let rows_per_col = 4;
        let cols = 2;
        for row in 0..rows_per_col {
            for col in 0..cols {
                let idx = top + row * cols + col;
                if idx >= items.len() { continue; }
                let (name, icon, price, owned) = &items[idx];
                let y = ROW_START_Y + ROW_STEP * row as f32;
                let x = COL_ICON_X[col];
                let icon_ent = ecs.add_ui_sized(x, y, 0.8, 0.8, icon, device, queue);
                let text = if *owned {
                    "— Owned".to_string()
                } else {
                    format!("— ${}", price)
                };
                let label = tr.add_text(ecs, device, queue, &text, 36.0, COL_LABEL_X[col], y, 4.0, 1.0, WHITE);
                self.rows.push(ShopRow { icon: icon_ent, label, label_key: None, name: name.clone(), item_idx: idx, x: COL_LABEL_X[col], y });
            }
        }
    }

    fn clear_rows(&mut self, ecs: &mut EcsAdapter) {
        for r in self.rows.drain(..) {
            ecs.delete_entity(r.icon);
            ecs.delete_entity(r.label);
        }
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        destroy_panel(ecs, &mut self.panel);
        self.clear_rows(ecs);
        if let Some(ent) = self.title.take() {
            ecs.delete_entity(ent);
        }
    }

    pub fn refresh(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        self.build_rows(ecs, tr, device, queue, items);
    }

    pub fn scroll_input(&mut self, input: &dyn InputSource, items_len: usize, window_size: (f32, f32)) {
        if !self.open || items_len == 0 { return; }
        let max = Self::max_scroll(items_len);

        let (_, wy) = input.scroll_diff();
        if wy.abs() > 0.01 {
            let raw = self.scroll_offset - wy * 4.0;
            self.scroll_offset = if wy > 0.0 {
                (raw / 8.0).floor() * 8.0
            } else {
                (raw / 8.0).ceil() * 8.0
            };
            self.scroll_offset = self.scroll_offset.clamp(0.0, max);
            self.touch_scroll_start = None;
            return;
        }

        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
            let in_panel = wx.abs() < self.panel.w / 2.0 && wy.abs() < self.panel.h / 2.0;
            let pressing = input.mouse_held(winit::event::MouseButton::Left);
            if pressing && in_panel {
                match self.touch_scroll_start {
                    Some((_, start_y)) => {
                        let dy = wy - start_y;
                        let raw = self.touch_scroll_base + dy / (-ROW_STEP) * 2.0;
                        self.scroll_offset = ((raw / 8.0).floor() * 8.0).clamp(0.0, max);
                    }
                    None => {
                        self.touch_scroll_start = Some((wx, wy));
                        self.touch_scroll_base = self.scroll_offset;
                    }
                }
            } else {
                self.touch_scroll_start = None;
            }
        } else {
            self.touch_scroll_start = None;
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
