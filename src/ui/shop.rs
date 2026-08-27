// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Окно магазина: покупка доступа к «необычным» (светящимся) предметам.
//  Каждый мир помнит, что именно куплено (ShopOwned), состояние
//  сохраняется в файле мира. Элементы создаются только при открытии
//  и уничтожаются при закрытии.
// ========================================================================

use specs::Entity;
use crate::ui::{Panel, create_panel, destroy_panel};
use crate::ui::text_renderer::TextRenderer;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::input::platform::InputSource;
use crate::ui::system::is_clicked;

/// Раскладка строк каталога в окне магазина.
const ROW_START_Y: f32 = 1.4;
const ROW_STEP: f32 = -1.1;
const ROW_HALF_W: f32 = 4.4;
const ROW_HALF_H: f32 = 0.5;
const ICON_X: f32 = -3.2;
const LABEL_X: f32 = -2.2;

/// Одна строка каталога: иконка предмета, подпись и имя объекта.
struct ShopRow {
    icon: Entity,
    label: Entity,
    label_key: Option<u64>,
    name: String,
    y: f32,
}

/// Состояние окна магазина и его UI-элементы.
pub struct Shop {
    pub open: bool,
    pub panel: Panel,
    title: Option<Entity>,
    rows: Vec<ShopRow>,
}

/// Элемент каталога: (имя объекта, путь к иконке, цена, куплено ли уже).
pub type ShopItem = (String, String, i32, bool);

impl Shop {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 9.0, 6.0, 0.85),
            title: None,
            rows: Vec::new(),
        }
    }

    /// Открывает окно и строит строки каталога.
    pub fn open(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        if self.open { return; }
        self.open = true;
        create_panel(ecs, device, queue, &mut self.panel);
        self.title = Some(tr.add_text(ecs, device, queue, "Shop", 64.0, 0.0, 2.4, 4.0, 2.0, WHITE));
        self.build_rows(ecs, tr, device, queue, items);
    }

    /// Перестраивает строки (вызывается при открытии и после покупки).
    fn build_rows(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        self.clear_rows(ecs);
        for (i, (name, icon, price, owned)) in items.iter().enumerate() {
            let y = ROW_START_Y + ROW_STEP * i as f32;
            let icon_ent = ecs.add_ui_sized(ICON_X, y, 0.8, 0.8, icon, device, queue);
            let text = if *owned {
                format!("{}  —  Owned", name)
            } else {
                format!("{}  —  ${}", name, price)
            };
            let label = tr.add_text(ecs, device, queue, &text, 36.0, LABEL_X, y, 6.0, 1.0, WHITE);
            self.rows.push(ShopRow { icon: icon_ent, label, label_key: None, name: name.clone(), y });
        }
    }

    fn clear_rows(&mut self, ecs: &mut EcsAdapter) {
        for r in self.rows.drain(..) {
            ecs.delete_entity(r.icon);
            ecs.delete_entity(r.label);
        }
    }

    /// Закрывает окно и убирает все созданные сущности.
    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        destroy_panel(ecs, &mut self.panel);
        self.clear_rows(ecs);
        if let Some(ent) = self.title.take() {
            ecs.delete_entity(ent);
        }
    }

    /// Обновляет подписи строк (например, после покупки предмет становится Owned).
    pub fn refresh(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        for (i, (name, _icon, price, owned)) in items.iter().enumerate() {
            if let Some(r) = self.rows.get_mut(i) {
                let text = if *owned {
                    format!("{}  —  Owned", name)
                } else {
                    format!("{}  —  ${}", name, price)
                };
                let (e, k) = tr.set_text(ecs, device, queue, Some(r.label), r.label_key, &text, 36.0, LABEL_X, r.y, 6.0, 1.0, WHITE);
                if let Some(e) = e { r.label = e; }
                r.label_key = k;
            }
        }
    }

    /// Возвращает индекс строки, по которой кликнули (или None).
    pub fn row_clicked(&self, input: &dyn InputSource, window_size: (f32, f32)) -> Option<usize> {
        if !self.open { return None; }
        for (i, r) in self.rows.iter().enumerate() {
            if is_clicked(input, window_size, 0.0, r.y, ROW_HALF_W, ROW_HALF_H) {
                return Some(i);
            }
        }
        None
    }
}
