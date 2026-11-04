// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use crate::ui::{Panel, destroy_panel};
use crate::ui::anim::{AnimEvent, PanelAnim};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;
use crate::ui::system::{is_clicked, is_inside, ndc_to_ui};

const ROW_START_Y: f32 = 1.4;
const ROW_STEP: f32 = -1.1;
const ROW_HALF_H: f32 = 0.5;
const COL_ICON_X: [f32; 2] = [-3.2, 1.8];
const COL_LABEL_X: [f32; 2] = [-2.0, 3.0];
const COL_HALF_W: f32 = 2.2;

const SCROLLBAR_X: f32 = 4.0;
const SCROLLBAR_W: f32 = 0.25;
const SCROLLBAR_TOP: f32 = 2.0;
const SCROLLBAR_BOT: f32 = -2.5;
const SCROLLBAR_H: f32 = 4.0;

// Диалог подтверждения покупки: размеры панели и кнопок.
const CONFIRM_W: f32 = 5.6;
const CONFIRM_H: f32 = 3.2;
const CONFIRM_BTN_Y: f32 = -1.0;
const CONFIRM_BTN_W: f32 = 2.2;
const CONFIRM_BTN_H: f32 = 0.8;
const CONFIRM_BUY_X: f32 = -1.35;
const CONFIRM_CLOSE_X: f32 = 1.35;

struct ShopRow {
    icon: Entity,
    label: Entity,
    item_idx: usize,
    x: f32,
    y: f32,
}

/// Что выбрал игрок в диалоге подтверждения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Buy,
    Close,
}

/// Диалог «купить или закрыть» поверх каталога магазина.
struct ConfirmDialog {
    dim: Entity,
    panel: Entity,
    title: Entity,
    name_label: Entity,
    price_label: Entity,
    buy_bg: Entity,
    buy_label: Entity,
    close_bg: Entity,
    close_label: Entity,
    item_name: String,
    price: i32,
}

pub struct Shop {
    pub open: bool,
    pub panel: Panel,
    rows: Vec<ShopRow>,
    pub scroll_offset: f32,
    touch_scroll_start: Option<(f32, f32)>,
    touch_scroll_base: f32,
    track: Option<Entity>,
    thumb: Option<Entity>,
    pub thumb_dragging: bool,
    thumb_grab_y: f32,
    thumb_grab_offset: f32,
    anim: PanelAnim,
    confirm: Option<ConfirmDialog>,
}

pub type ShopItem = (String, String, i32, bool);

pub(crate) fn max_scroll(total: usize) -> f32 {
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
            let (_, icon, price, owned) = &items[idx];
            let y = ROW_START_Y + ROW_STEP * row as f32;
            let x = COL_ICON_X[col];
            let icon_ent = ecs.add_ui_sized(x, y, 0.8, 0.8, icon, device, queue);
            let text = if *owned { "— Owned".to_string() } else { format!("— ${}", price) };
            let label = tr.add_text(ecs, device, queue, &text, 36.0, COL_LABEL_X[col], y, 4.0, 2.0, WHITE);
            rows.push(ShopRow { icon: icon_ent, label, item_idx: idx, x: COL_LABEL_X[col], y });
        }
    }
}

pub(crate) fn thumb_y(scroll_offset: f32, total: usize) -> f32 {
    let max = max_scroll(total);
    if max <= 0.0 { return (SCROLLBAR_TOP + SCROLLBAR_BOT) / 2.0; }
    let h = thumb_height(total);
    let pad = h / 2.0;
    let t = scroll_offset / max;
    (SCROLLBAR_TOP - pad) - t * (SCROLLBAR_H - h)
}

pub(crate) fn thumb_height(total: usize) -> f32 {
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
        let on_track = (wx - SCROLLBAR_X).abs() < SCROLLBAR_W * 4.0 && wy >= SCROLLBAR_BOT - 0.5 && wy <= SCROLLBAR_TOP + 0.5;
        let pressing = input.mouse_held(winit::event::MouseButton::Left);

        if pressing && (on_track || *thumb_dragging) {
            if !*thumb_dragging {
                *thumb_dragging = true;
                *thumb_grab_y = wy;
                *thumb_grab_offset = *offset;
            }
            let dy = *thumb_grab_y - wy;
            *offset = (*thumb_grab_offset + dy / (SCROLLBAR_H * 0.8) * max).clamp(0.0, max);
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
            rows: Vec::new(),
            scroll_offset: 0.0,
            touch_scroll_start: None,
            touch_scroll_base: 0.0,
            track: None,
            thumb: None,
            thumb_dragging: false,
            thumb_grab_y: 0.0,
            thumb_grab_offset: 0.0,
            anim: PanelAnim::new(),
            confirm: None,
        }
    }

    // Все живые сущности панели — для анимации закрытия/открытия.
    fn collect_entities(&self) -> Vec<Entity> {
        let mut v = Vec::new();
        if let Some(e) = self.panel.entity {
            v.push(e);
        }
        if let Some(e) = self.track {
            v.push(e);
        }
        if let Some(e) = self.thumb {
            v.push(e);
        }
        for r in &self.rows {
            v.push(r.icon);
            v.push(r.label);
        }
        v
    }

    // Убирает сущности и обнуляет поля (вызывается по завершении закрытия).
    fn destroy_content(&mut self, ecs: &mut EcsAdapter) {
        self.close_confirm(ecs);
        destroy_panel(ecs, &mut self.panel);
        clear_rows(ecs, &mut self.rows);
        if let Some(e) = self.track.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.thumb.take() {
            ecs.delete_entity(e);
        }
    }

    pub fn open(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        if self.open { return; }
        self.close_confirm(ecs);
        if self.anim.is_active() {
            self.anim.cancel(ecs);
            self.destroy_content(ecs);
        }
        self.open = true;
        self.scroll_offset = 0.0;
        self.touch_scroll_start = None;
        self.thumb_dragging = false;
        self.thumb_grab_y = 0.0;
        self.thumb_grab_offset = 0.0;
        let ent = ecs.add_ui_sized(self.panel.x, self.panel.y, self.panel.w, self.panel.h, "assets/tex/ui/shop_panel.png", device, queue);
        ecs.update_sprite_alpha(ent, self.panel.alpha);
        self.panel.entity = Some(ent);

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

        let ents = self.collect_entities();
        self.anim.start_open(ecs, ents);
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        self.close_confirm(ecs);
        self.thumb_dragging = false;
        self.touch_scroll_start = None;
        let ents = self.collect_entities();
        self.anim.start_close(ecs, ents);
    }

    /// Ежекадровая анимация окна: вызывает PanelAnim и подчищает поля.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.anim.tick(ecs, dt) == AnimEvent::Closed {
            self.destroy_content(ecs);
            self.scroll_offset = 0.0;
            self.thumb_dragging = false;
            self.touch_scroll_start = None;
            self.touch_scroll_base = 0.0;
        }
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

    // ====================================================================
    //  Диалог подтверждения покупки
    // ====================================================================

    pub fn confirming(&self) -> bool {
        self.confirm.is_some()
    }

    /// Предмет, ожидающий подтверждения: (имя, цена).
    pub fn pending_purchase(&self) -> Option<(String, i32)> {
        self.confirm.as_ref().map(|c| (c.item_name.clone(), c.price))
    }

    /// Открывает диалог «купить или закрыть» для выбранного предмета.
    pub fn open_confirm(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, name: &str, price: i32) {
        self.close_confirm(ecs);

        // Затемнение каталога: диалог должен читаться как модальное окно.
        let dim = ecs.add_ui_sized(0.0, 0.0, 24.0, 16.0, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(dim, 0.35);
        ecs.update_transform_z(dim, Z_UI_DIALOG_DIM);

        // Отдельная текстура без заголовка «SHOP» и с пропорциями под диалог.
        let panel = ecs.add_ui_sized(0.0, 0.0, CONFIRM_W, CONFIRM_H, "assets/tex/ui/confirm_panel.png", device, queue);
        ecs.update_transform_z(panel, Z_UI_DIALOG);

        let title = tr.add_text(ecs, device, queue, "Buy item?", 48.0, 0.0, 1.05, 4.0, 2.0, GOLD);
        ecs.update_transform_z(title, Z_UI_DIALOG_TEXT);
        let mut display_name = name.replace('_', " ");
        if let Some(c) = display_name.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        let name_label = tr.add_text(ecs, device, queue, &display_name, 40.0, 0.0, 0.4, 4.8, 1.5, WHITE);
        ecs.update_transform_z(name_label, Z_UI_DIALOG_TEXT);
        let price_text = format!("Price: ${}", price);
        let price_label = tr.add_text(ecs, device, queue, &price_text, 36.0, 0.0, -0.1, 3.0, 1.5, WHITE);
        ecs.update_transform_z(price_label, Z_UI_DIALOG_TEXT);

        let buy_bg = ecs.add_ui_sized(CONFIRM_BUY_X, CONFIRM_BTN_Y, CONFIRM_BTN_W, CONFIRM_BTN_H, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(buy_bg, 0.9);
        ecs.update_transform_z(buy_bg, Z_UI_DIALOG);
        let buy_label = tr.add_text(ecs, device, queue, "Buy", 44.0, CONFIRM_BUY_X, CONFIRM_BTN_Y, 1.5, 1.5, GREEN);
        ecs.update_transform_z(buy_label, Z_UI_DIALOG_TEXT);

        let close_bg = ecs.add_ui_sized(CONFIRM_CLOSE_X, CONFIRM_BTN_Y, CONFIRM_BTN_W, CONFIRM_BTN_H, "assets/tex/dev_tools/black.png", device, queue);
        ecs.update_sprite_alpha(close_bg, 0.9);
        ecs.update_transform_z(close_bg, Z_UI_DIALOG);
        let close_label = tr.add_text(ecs, device, queue, "Close", 44.0, CONFIRM_CLOSE_X, CONFIRM_BTN_Y, 1.5, 1.5, WHITE);
        ecs.update_transform_z(close_label, Z_UI_DIALOG_TEXT);

        self.confirm = Some(ConfirmDialog {
            dim, panel, title, name_label, price_label,
            buy_bg, buy_label, close_bg, close_label,
            item_name: name.to_string(),
            price,
        });
    }

    /// Удаляет сущности диалога; безопасно вызывать без открытого диалога.
    pub fn close_confirm(&mut self, ecs: &mut EcsAdapter) {
        let Some(c) = self.confirm.take() else { return };
        for e in [
            c.dim, c.panel, c.title, c.name_label, c.price_label,
            c.buy_bg, c.buy_label, c.close_bg, c.close_label,
        ] {
            ecs.delete_entity(e);
        }
    }

    /// true, если точка (wx, wy) внутри панели диалога.
    fn confirm_hit(&self, wx: f32, wy: f32) -> bool {
        is_inside(wx, wy, 0.0, 0.0, CONFIRM_W / 2.0, CONFIRM_H / 2.0)
    }

    /// Опрос диалога: Buy / Close (в том числе клик вне панели диалога).
    pub fn confirm_action(&self, input: &dyn InputSource, window_size: (f32, f32)) -> Option<ConfirmAction> {
        self.confirm.as_ref()?;
        if is_clicked(input, window_size, CONFIRM_BUY_X, CONFIRM_BTN_Y, CONFIRM_BTN_W / 2.0, CONFIRM_BTN_H / 2.0) {
            return Some(ConfirmAction::Buy);
        }
        if is_clicked(input, window_size, CONFIRM_CLOSE_X, CONFIRM_BTN_Y, CONFIRM_BTN_W / 2.0, CONFIRM_BTN_H / 2.0) {
            return Some(ConfirmAction::Close);
        }
        if input.mouse_pressed(winit::event::MouseButton::Left) {
            let Some((mx, my)) = input.cursor() else { return None };
            let (wx, wy) = ndc_to_ui(mx, my, window_size);
            if !self.confirm_hit(wx, wy) {
                return Some(ConfirmAction::Close);
            }
        }
        None
    }
}
