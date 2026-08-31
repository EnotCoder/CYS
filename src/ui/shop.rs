// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use crate::ui::{Panel, destroy_panel};
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
const LABEL_X: f32 = -2.0;

/// Границы панели магазина (мировые координаты).
const PANEL_TOP: f32 = 3.0;
const PANEL_BOTTOM: f32 = -3.0;

/// Одна строка каталога: иконка предмета, подпись и имя объекта.
struct ShopRow {
    icon: Entity,
    label: Entity,
    label_key: Option<u64>,
    name: String,
    /// Индекс этого предмета в полном каталоге (items).
    item_idx: usize,
    y: f32,
}

/// Состояние окна магазина и его UI-элементы.
pub struct Shop {
    pub open: bool,
    pub panel: Panel,
    title: Option<Entity>,
    rows: Vec<ShopRow>,
    /// Плавный скролл: какая доля предмета видна сверху (0 = первый предмет
    /// прижат к верхней границе, большее значение = список сдвинут вниз).
    pub scroll_offset: f32,
    /// Состояние touch-скролла: позиция касания при начале перетаскивания.
    touch_scroll_start: Option<(f32, f32)>,
    /// scroll_offset в момент начала touch-перетаскивания.
    touch_scroll_base: f32,
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
            scroll_offset: 0.0,
            touch_scroll_start: None,
            touch_scroll_base: 0.0,
        }
    }

    /// Максимальное значение scroll_offset для данного числа предметов.
    fn max_scroll(total: usize) -> f32 {
        if total <= 1 { 0.0 } else { (total as f32) - 1.0 }
    }

    /// Открывает окно и строит строки каталога.
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

    /// Перестраивает видимые строки (вызывается при открытии, скролле и покупке).
    fn build_rows(&mut self, ecs: &mut EcsAdapter, tr: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, items: &[ShopItem]) {
        self.clear_rows(ecs);
        let top = self.scroll_offset;
        // Панель: y ∈ [-3, 3], строка: y = 1.4 - 1.1*(i - top).
        // Строка видна целиком, если y ± 0.5 ⊂ [-3, 3] → y ∈ [-2.5, 2.5]:
        //   -2.5 ≤ 1.4 - 1.1*(i - top) ≤ 2.5
        //   top - 1.0 ≤ i ≤ top + 3.5
        let min_i = (top - 1.0).ceil() as i32;
        let max_i = (top + 3.5).floor() as i32;
        for (i, (name, icon, price, owned)) in items.iter().enumerate() {
            if (i as i32) < min_i || (i as i32) > max_i { continue; }
            let y = ROW_START_Y + ROW_STEP * (i as f32 - top);
            let icon_ent = ecs.add_ui_sized(ICON_X, y, 0.8, 0.8, icon, device, queue);
            let text = if *owned {
                "— Owned".to_string()
            } else {
                format!("— ${}", price)
            };
            let label = tr.add_text(ecs, device, queue, &text, 36.0, LABEL_X, y, 6.0, 1.0, WHITE);
            self.rows.push(ShopRow { icon: icon_ent, label, label_key: None, name: name.clone(), item_idx: i, y });
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
        self.build_rows(ecs, tr, device, queue, items);
    }

    /// Обрабатывает скролл: колесо мыши (desktop) и перетаскивание пальцем
    /// (touch). Вызывается каждый кадр, пока магазин открыт.
    pub fn scroll_input(&mut self, input: &dyn InputSource, items_len: usize, window_size: (f32, f32)) {
        if !self.open || items_len == 0 { return; }
        let max = Self::max_scroll(items_len);

        // Mouse wheel: scroll_diff().1 > 0 = вверх (к первому элементу)
        let ( _, wy) = input.scroll_diff();
        if wy.abs() > 0.01 {
            self.scroll_offset = (self.scroll_offset - wy).clamp(0.0, max);
            self.touch_scroll_start = None;
            return;
        }

        // Touch drag: один палец внутри панели — drag вертикально = скролл.
        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
            let in_panel = wx.abs() < self.panel.w / 2.0 && wy.abs() < self.panel.h / 2.0;
            let pressing = input.mouse_held(winit::event::MouseButton::Left);
            if pressing && in_panel {
                match self.touch_scroll_start {
                    Some((_, start_y)) => {
                        let dy = wy - start_y;
                        let items_per_screen = self.panel.h / (-ROW_STEP);
                        self.scroll_offset = (self.touch_scroll_base + dy / (-ROW_STEP) * items_per_screen).clamp(0.0, max);
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

    /// Возвращает индекс строки, по которой кликнули (или None).
    /// Возвращает индекс в полном каталоге items (не в видимых строках).
    pub fn row_clicked(&self, input: &dyn InputSource, window_size: (f32, f32)) -> Option<usize> {
        if !self.open { return None; }
        for r in self.rows.iter() {
            if is_clicked(input, window_size, 0.0, r.y, ROW_HALF_W, ROW_HALF_H) {
                return Some(r.item_idx);
            }
        }
        None
    }
}
