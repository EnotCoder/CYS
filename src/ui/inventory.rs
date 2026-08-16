// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use crate::EcsAdapter;
use crate::core::constants::*;

// ========================================================================
//  Inventory — управление инвентарём (сетка, табы, курсор)
// ========================================================================

pub struct Inventory {
    // Открыт ли инвентарь и находится ли игра в режиме его использования
    pub open: bool,
    pub mode: bool,
    // Номер выбранной ячейки сетки (по строкам*колоннам); INV_NONE — нет выбора
    pub selected: i32,
    // Текущий таб категории (обычные, ковры, настенный декор, уличное)
    pub tab: i32,
    // UI-сущности сетки предметов и закладок табов (нужны для скрытия/показа)
    grid_entities: Vec<Entity>,
    tab_entities: Vec<Entity>,
    // Анимация открытия: отсчёт времени и активность «попа» появления
    pop_timer: f64,
    popping: bool,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            open: false,
            mode: false,
            selected: INV_NONE,
            tab: 0,
            grid_entities: Vec::new(),
            tab_entities: Vec::new(),
            pop_timer: 0.0,
            popping: false,
        }
    }

    // Полный сброс состояния — используется при смене сцены/загрузке
    pub fn reset(&mut self) {
        self.open = false;
        self.mode = false;
        self.selected = INV_NONE;
        self.tab = 0;
        self.grid_entities.clear();
        self.tab_entities.clear();
        self.pop_timer = 0.0;
        self.popping = false;
    }

    // ================================================================
    //  Открытие / закрытие
    // ================================================================

    // Открыть инвентарь: сбросить выбор, показать сетку и табы
    pub fn enter(&mut self, ecs: &mut EcsAdapter) {
        self.tab = 0;
        self.selected = INV_NONE;
        self.show_grid(ecs);
        self.show_tabs(ecs);
        self.open = true;
        self.mode = true;
        self.start_pop();
    }

    // Закрыть: убрать UI-элементы сетки и табов из ECS
    pub fn exit(&mut self, ecs: &mut EcsAdapter) {
        self.hide_grid(ecs);
        self.hide_tabs(ecs);
        self.open = false;
        self.mode = false;
        self.popping = false;
    }

    // Стартуем анимацию появления только для отрисуемых сейчас элементов.
    fn start_pop(&mut self) {
        self.pop_timer = 0.0;
        self.popping = true;
    }

    /// Ежекадровая анимация открытия: иконки «подпрыгивают» с каскадной задержкой
    /// (easeOutBack). Вызывается из update() игровой сцены.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if !self.popping {
            return;
        }
        self.pop_timer += dt;
        // Каскад: каждая иконка стартует чуть позже предыдущей.
        let dur = 0.35;
        let mut all_done = true;
        for (i, ent) in self.grid_entities.iter().enumerate() {
            let t = ((self.pop_timer - i as f64 * 0.02) / dur).clamp(0.0, 1.0) as f32;
            if t < 1.0 {
                all_done = false;
            }
            let s = 0.6 + 0.4 * crate::core::util::ease_out_back(t);
            ecs.update_sprite_scale(*ent, s);
        }
        for (i, ent) in self.tab_entities.iter().enumerate() {
            let t = ((self.pop_timer - i as f64 * 0.03) / dur).clamp(0.0, 1.0) as f32;
            if t < 1.0 {
                all_done = false;
            }
            let s = 0.6 + 0.4 * crate::core::util::ease_out_back(t);
            ecs.update_sprite_scale(*ent, s);
        }
        if all_done {
            self.popping = false;
        }
    }

    // ================================================================
    //  Предметы текущей вкладки
    // ================================================================

    // Список имён предметов для активной вкладки
    pub fn items(&self) -> &'static [&'static str] {
        match self.tab {
            0 => INV_REGULAR,
            1 => INV_CARPETS,
            2 => INV_WALLDECOR,
            _ => INV_OUTDOOR,
        }
    }

    // ================================================================
    //  Получение имени предмета под курсором
    // ================================================================

    // Имя предмета в выбранной ячейке сетки (если ячейка не пустая)
    pub fn selected_item_name(&self) -> Option<&'static str> {
        let row = self.selected / INVENTORY_COLS;
        let col = self.selected % INVENTORY_COLS;
        let item_idx = crate::core::util::inventory_index(row, col) as usize;
        self.items().get(item_idx).copied()
    }

    // ================================================================
    //  Переключение вкладки
    // ================================================================

    // Переключить таб: перерисовать сетку, подсветить активный таб, сбросить выбор
    pub fn switch_tab(&mut self, new_tab: i32, ecs: &mut EcsAdapter) {
        self.tab = new_tab;
        self.hide_grid(ecs);
        self.show_grid(ecs);
        // Активный таб — насыщенный, остальные — полупрозрачные
        for (i, ent) in self.tab_entities.iter().enumerate() {
            let a = if i as i32 == self.tab { 1.0 } else { 0.5 };
            ecs.update_sprite_alpha(*ent, a);
        }
        self.selected = INV_NONE;
    }

    // ================================================================
    //  Перенос предмета на панель
    // ================================================================

    // Положить выбранный предмет инвентаря в активный слот хотбара
    pub fn transfer_to_slot(
        &mut self,
        ecs: &mut EcsAdapter,
        act_slot: usize,
        hotbar_slots: &mut [crate::data::Slot],
        hotbar_entities: &[Entity],
    ) {
        let Some(name) = self.selected_item_name() else { return };
        let new_slot = crate::data::make_slot(name);
        if act_slot < hotbar_slots.len() {
            hotbar_slots[act_slot] = new_slot;
            // Обновляем иконку слота на UI согласно выбранному предмету
            if act_slot < hotbar_entities.len() {
                let path = crate::core::util::slot_icon_path(name);
                ecs.update_sprite_texture(hotbar_entities[act_slot], &path);
            }
        }
        // После переноса инвентарь закрывается
        self.exit(ecs);
    }

    // ================================================================
    //  Обработка клика по сетке инвентаря
    //  Возвращает true, если нужно сделать transfer
    // ================================================================

    // Клик по ячейке сетки: сохраняет выбранную ячейку, если она в пределах сетки
    pub fn handle_grid_click(&mut self, col: i32, row: i32) -> bool {
        if col < 0 || col >= INVENTORY_COLS || row < 0 || row >= INVENTORY_ROWS {
            return false;
        }
        self.selected = row * INVENTORY_COLS + col;
        true
    }

    // ================================================================
    //  Рендер сетки
    // ================================================================

    // Создаём UI-иконки всех предметов сетки (снизу вверх); пустые ячейки — null.png
    fn show_grid(&mut self, ecs: &mut EcsAdapter) {
        let items = self.items();
        for row in (0..INVENTORY_ROWS).rev() {
            for col in 0..INVENTORY_COLS {
                let item_idx = crate::core::util::inventory_index(row, col) as usize;
                let tex = if item_idx < items.len() {
                    crate::core::util::slot_icon_path(items[item_idx])
                } else {
                    format!("{}{}", TEX_UI_ICON_SLOTS_MAP_DIR, "null.png")
                };
                let ent = ecs.add_ui(
                    SLOT_BAR_X + col as f32,
                    INVENTORY_BASE_Y + row as f32,
                    &tex,
                );
                self.grid_entities.push(ent);
            }
        }
    }

    // Убираем все UI-иконки сетки из ECS
    fn hide_grid(&mut self, ecs: &mut EcsAdapter) {
        let removed: Vec<Entity> = self.grid_entities.drain(..).collect();
        ecs.delete_entities(&removed);
    }

    // ================================================================
    //  Рендер табов
    // ================================================================

    // Создаём закладки категорий; активная подсвечивается полностью
    fn show_tabs(&mut self, ecs: &mut EcsAdapter) {
        for (i, tex) in TAB_TEX.iter().enumerate() {
            let ent = ecs.add_ui(SLOT_BAR_X + i as f32, INV_TAB_Y, tex);
            let a = if i as i32 == self.tab { 1.0 } else { 0.5 };
            ecs.update_sprite_alpha(ent, a);
            self.tab_entities.push(ent);
        }
    }

    // Убираем закладки из ECS
    fn hide_tabs(&mut self, ecs: &mut EcsAdapter) {
        let removed: Vec<Entity> = self.tab_entities.drain(..).collect();
        ecs.delete_entities(&removed);
    }
}
