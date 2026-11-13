// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::Entity;
use specs::WorldExt;
use crate::EcsAdapter;
use crate::core::constants::*;
use crate::ecs::components::{ShopDenied, ShopOwned};

// ========================================================================
//  Inventory — управление инвентарём (сетка, табы, курсор)
// ========================================================================

/// UI-элемент сетки/таба с базовой позицией: сдвиг к нижнему краю экрана
/// применяется поверх неё, поэтому анимации и повторные вызовы не «дрейфуют».
#[derive(Clone, Copy)]
struct PlacedEntity {
    ent: Entity,
    base_x: f32,
    base_y: f32,
}

pub struct Inventory {
    // Открыт ли инвентарь и находится ли игра в режиме его использования
    pub open: bool,
    pub mode: bool,
    // Номер выбранной ячейки сетки (по строкам*колоннам); INV_NONE — нет выбора
    pub selected: i32,
    // Текущий таб категории (обычные, ковры, настенный декор, уличное)
    pub tab: i32,
    // Текущая страница вкладок: по TAB_PAGE_SIZE иконок на страницу
    pub tab_page: i32,
    // Сдвиг сетки по вертикали: держит её вплотную над хотбаром, когда тот
    // прижат к нижнему краю экрана.
    bottom_dy: f32,
    // UI-сущности сетки предметов и закладок табов (нужны для скрытия/показа)
    grid_entities: Vec<PlacedEntity>,
    tab_entities: Vec<PlacedEntity>,
    // Кнопки листания страниц вкладок (слева и справа от ряда табов)
    page_entities: Vec<PlacedEntity>,
    // Замочки на некупленных предметах (значок в углу ячейки)
    lock_entities: Vec<PlacedEntity>,
    // Анимация открытия: отсчёт времени и активность «попа» появления
    pop_timer: f64,
    popping: bool,
    // Анимация закрытия: сжатие + затухание перед удалением сущностей
    close_timer: f64,
    closing: bool,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            open: false,
            mode: false,
            selected: INV_NONE,
            tab: 0,
            tab_page: 0,
            bottom_dy: 0.0,
            grid_entities: Vec::new(),
            tab_entities: Vec::new(),
            page_entities: Vec::new(),
            lock_entities: Vec::new(),
            pop_timer: 0.0,
            popping: false,
            close_timer: 0.0,
            closing: false,
        }
    }

    // Полный сброс состояния — используется при смене сцены/загрузке
    pub fn reset(&mut self) {
        self.open = false;
        self.mode = false;
        self.selected = INV_NONE;
        self.tab = 0;
        self.tab_page = 0;
        self.grid_entities.clear();
        self.tab_entities.clear();
        self.page_entities.clear();
        self.lock_entities.clear();
        self.pop_timer = 0.0;
        self.popping = false;
        self.close_timer = 0.0;
        self.closing = false;
    }

    // ================================================================
    //  Открытие / закрытие
    // ================================================================

    // Открыть инвентарь: сбросить выбор, показать сетку и табы
    pub fn enter(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Если ещё идёт анимация закрытия — доводим её до конца (удаляем остатки)
        if self.closing {
            self.hide_grid(ecs);
            self.hide_tabs(ecs);
            self.closing = false;
        }
        self.tab = 0;
        self.tab_page = 0;
        self.selected = INV_NONE;
        self.show_grid(ecs, device, queue);
        self.show_tabs(ecs);
        self.open = true;
        self.mode = true;
        self.start_pop();
    }

    // Закрыть: запустить анимацию сжатия; сущности удаляются в tick()
    pub fn exit(&mut self, _ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        self.mode = false;
        self.popping = false;
        self.close_timer = 0.0;
        self.closing = true;
    }

    // Стартуем анимацию появления только для отрисуемых сейчас элементов.
    fn start_pop(&mut self) {
        self.pop_timer = 0.0;
        self.popping = true;
    }

    /// Ежекадровая анимация открытия: иконки «подпрыгивают» с каскадной задержкой
    /// (easeOutBack). Вызывается из update() игровой сцены.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        if self.popping {
            self.tick_pop(ecs, dt);
        }
        if self.closing {
            self.tick_close(ecs, dt);
        }
    }

    fn tick_pop(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        self.pop_timer += dt;
        // Каскад: каждая иконка стартует чуть позже предыдущей.
        let dur = 0.35;
        let mut all_done = true;
        for (i, p) in self.grid_entities.iter().enumerate() {
            let t = ((self.pop_timer - i as f64 * 0.02) / dur).clamp(0.0, 1.0) as f32;
            if t < 1.0 {
                all_done = false;
            }
            let s = 0.6 + 0.4 * crate::core::util::ease_out_back(t);
            ecs.update_sprite_scale(p.ent, s);
        }
        for (i, p) in self.tab_entities.iter().chain(self.page_entities.iter()).enumerate() {
            let t = ((self.pop_timer - i as f64 * 0.03) / dur).clamp(0.0, 1.0) as f32;
            if t < 1.0 {
                all_done = false;
            }
            let s = 0.6 + 0.4 * crate::core::util::ease_out_back(t);
            ecs.update_sprite_scale(p.ent, s);
        }
        if all_done {
            self.popping = false;
        }
    }

    /// Анимация закрытия: плавное сжатие и затухание, затем удаление сущностей.
    fn tick_close(&mut self, ecs: &mut EcsAdapter, dt: f64) {
        self.close_timer += dt;
        let t = (self.close_timer / 0.3).clamp(0.0, 1.0) as f32;
        let e = t * t;
        let s = 1.0 - 0.3 * e;
        let a = 1.0 - e;
        for p in self.grid_entities.iter()
            .chain(self.tab_entities.iter())
            .chain(self.page_entities.iter())
        {
            ecs.update_sprite_scale(p.ent, s);
            ecs.update_sprite_alpha(p.ent, a);
        }
        for p in &self.lock_entities {
            ecs.update_sprite_alpha(p.ent, a);
        }
        if t >= 1.0 {
            self.hide_grid(ecs);
            self.hide_tabs(ecs);
            self.closing = false;
            self.selected = INV_NONE;
        }
    }

    // ================================================================
    //  Предметы текущей вкладки
    // ================================================================

    // Список имён предметов для активной вкладки
    pub fn items(&self) -> &'static [&'static str] {
        crate::data::inventory_tab_names(self.tab)
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

    // Переключить таб: перерисовать страницу вкладок, сетку и сбросить выбор
    pub fn switch_tab(&mut self, new_tab: i32, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        let last = crate::scripts::items::TAB_NAMES.len() as i32 - 1;
        self.tab = new_tab.clamp(0, last.max(0));
        self.tab_page = Self::tab_page_of(self.tab);
        self.hide_grid(ecs);
        self.show_grid(ecs, device, queue);
        // Страница могла смениться — пересобираем видимые табы и стрелки.
        self.hide_tabs(ecs);
        self.show_tabs(ecs);
        self.selected = INV_NONE;
    }

    /// Количество страниц вкладок (по TAB_PAGE_SIZE иконок на странице).
    pub fn tab_page_count() -> i32 {
        let total = crate::scripts::items::TAB_NAMES.len();
        total.div_ceil(TAB_PAGE_SIZE).max(1) as i32
    }

    /// Страница, на которой находится вкладка `tab`.
    pub fn tab_page_of(tab: i32) -> i32 {
        (tab / TAB_PAGE_SIZE as i32).clamp(0, Self::tab_page_count() - 1)
    }

    /// Сколько вкладок реально видно на текущей странице.
    pub fn visible_tab_count(&self) -> usize {
        let total = crate::scripts::items::TAB_NAMES.len();
        let start = self.tab_page.max(0) as usize * TAB_PAGE_SIZE;
        total.saturating_sub(start).min(TAB_PAGE_SIZE)
    }

    /// Сдвинуть страницу вкладок; true — страница действительно сменилась.
    pub fn switch_page(
        &mut self,
        delta: i32,
        ecs: &mut EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> bool {
        let next = (self.tab_page + delta).clamp(0, Self::tab_page_count() - 1);
        if next == self.tab_page {
            return false;
        }
        let first = next * TAB_PAGE_SIZE as i32;
        self.switch_tab(first, ecs, device, queue);
        true
    }

    /// Вкладка после шага `delta` по кругу (стрелки влево/вправо).
    pub fn tab_after(current: i32, delta: i32) -> i32 {
        let count = crate::scripts::items::TAB_NAMES.len() as i32;
        if count <= 0 {
            return 0;
        }
        (current + delta).rem_euclid(count)
    }

    // ================================================================
    //  Перенос предмета на панель
    // ================================================================

    // Положить выбранный предмет инвентаря в активный слот хотбара.
    // Некупленные предметы брать нельзя — выставляем ShopDenied (покажет
    // подсказку «You don't have this item») и инвентарь не закрываем.
    pub fn transfer_to_slot(
        &mut self,
        ecs: &mut EcsAdapter,
        act_slot: usize,
        hotbar_slots: &mut [crate::data::Slot],
        hotbar_entities: &[Entity],
    ) {
        let Some(name) = self.selected_item_name() else { return };
        let owned = ecs.world.read_resource::<ShopOwned>().0.iter().any(|o| o == name);
        if crate::data::placement::requires_shop(name) && !owned {
            ecs.world.write_resource::<ShopDenied>().0 = true;
            crate::audio::play("error");
            return;
        }
        let new_slot = crate::data::make_slot(name);
        if act_slot < hotbar_slots.len() {
            hotbar_slots[act_slot] = new_slot;
            // Обновляем иконку слота на UI согласно выбранному предмету
            if act_slot < hotbar_entities.len() {
                let path = crate::core::util::slot_icon_path(name);
                ecs.update_sprite_texture(hotbar_entities[act_slot], &path);
            }
        }
        // После переноса инвентарь НЕ закрывается (согласно новой логике)
        // self.exit(ecs);
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

    // Создаём UI-иконки всех предметов сетки (снизу вверх); пустые ячейки — null.png.
    // На некупленных предметах рисуем замок в углу (см. TEX_LOCK).
    fn show_grid(&mut self, ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue) {
        let items = self.items();
        for row in (0..INVENTORY_ROWS).rev() {
            for col in 0..INVENTORY_COLS {
                let item_idx = crate::core::util::inventory_index(row, col) as usize;
                let (tex, icon_size) = if item_idx < items.len() {
                    let name = items[item_idx];
                    (
                        crate::core::util::slot_icon_path(name),
                        crate::core::util::slot_icon_size(name),
                    )
                } else {
                    (
                        format!("{}{}", TEX_UI_ICON_SLOTS_MAP_DIR, "null.png"),
                        [1.0, 1.0],
                    )
                };
                let x = SLOT_BAR_X + col as f32;
                let y = INVENTORY_BASE_Y + row as f32 + self.bottom_dy;
                let ent = if (icon_size[0] - 1.0).abs() < f32::EPSILON
                    && (icon_size[1] - 1.0).abs() < f32::EPSILON
                {
                    ecs.add_ui(x, y, &tex)
                } else {
                    ecs.add_ui_sized(x, y, icon_size[0], icon_size[1], &tex, device, queue)
                };
                self.grid_entities.push(PlacedEntity { ent, base_x: SLOT_BAR_X + col as f32, base_y: INVENTORY_BASE_Y + row as f32 });

                // Доступ к такому предмету НЕ открыт (нужна покупка в магазине).
                if item_idx < items.len() {
                    let name = items[item_idx];
                    let owned = ecs.world.read_resource::<ShopOwned>().0.iter().any(|o| o == name);
                    if crate::data::placement::requires_shop(name) && !owned {
                        let lock = ecs.add_ui_sized(
                            x + LOCK_OFFSET_X, y + LOCK_OFFSET_Y,
                            LOCK_SIZE, LOCK_SIZE, TEX_LOCK, device, queue,
                        );
                        // Замок рисуется поверх иконки предмета (следующий под-слой Z_UI).
                        ecs.update_transform_z(lock, Z_UI_TEXT);
                        self.lock_entities.push(PlacedEntity {
                            ent: lock,
                            base_x: SLOT_BAR_X + col as f32 + LOCK_OFFSET_X,
                            base_y: INVENTORY_BASE_Y + row as f32 + LOCK_OFFSET_Y,
                        });
                    }
                }
            }
        }
    }

    /// Сдвиг сетки/табов по вертикали (прижатие хотбара к нижнему краю).
    pub fn set_bottom_offset(&mut self, dy: f32) {
        self.bottom_dy = dy;
    }

    /// Переставляет уже созданные элементы сетки/табов по актуальному сдвигу.
    pub fn apply_layout(&mut self, ecs: &mut EcsAdapter) {
        for p in self.grid_entities.iter()
            .chain(self.tab_entities.iter())
            .chain(self.page_entities.iter())
            .chain(self.lock_entities.iter())
        {
            ecs.update_transform_position(p.ent, p.base_x, p.base_y + self.bottom_dy);
        }
    }

    // Убираем все UI-иконки сетки из ECS
    fn hide_grid(&mut self, ecs: &mut EcsAdapter) {
        let removed: Vec<Entity> = self.grid_entities.drain(..).map(|p| p.ent).collect();
        ecs.delete_entities(&removed);
        let locks: Vec<Entity> = self.lock_entities.drain(..).map(|p| p.ent).collect();
        ecs.delete_entities(&locks);
    }

    // ================================================================
    //  Рендер табов
    // ================================================================

    // Создаём закладки текущей страницы и кнопки-стрелки по краям.
    // Активная вкладка — насыщенная, остальные — полупрозрачные.
    fn show_tabs(&mut self, ecs: &mut EcsAdapter) {
        let start = self.tab_page.max(0) * TAB_PAGE_SIZE as i32;
        let visible = self.visible_tab_count();
        for slot in 0..visible {
            let tab = start + slot as i32;
            let x = SLOT_BAR_X + slot as f32;
            let ent = ecs.add_ui(x, INV_TAB_Y + self.bottom_dy, TAB_TEX[tab as usize]);
            let a = if tab == self.tab { 1.0 } else { 0.5 };
            ecs.update_sprite_alpha(ent, a);
            self.tab_entities.push(PlacedEntity { ent, base_x: x, base_y: INV_TAB_Y });
        }
        if Self::tab_page_count() <= 1 {
            return;
        }

        // Левая стрелка — слева от первой вкладки, правая — после последней.
        let left_x = SLOT_BAR_X - 1.0;
        let right_x = SLOT_BAR_X + visible as f32;
        let left = ecs.add_ui(left_x, INV_TAB_Y + self.bottom_dy, TAB_ARROW_LEFT_TEX);
        let right = ecs.add_ui(right_x, INV_TAB_Y + self.bottom_dy, TAB_ARROW_RIGHT_TEX);
        let left_alpha = if self.tab_page > 0 { 1.0 } else { TAB_ARROW_DISABLED_ALPHA };
        let right_alpha = if self.tab_page + 1 < Self::tab_page_count() { 1.0 } else { TAB_ARROW_DISABLED_ALPHA };
        ecs.update_sprite_alpha(left, left_alpha);
        ecs.update_sprite_alpha(right, right_alpha);
        self.page_entities.push(PlacedEntity { ent: left, base_x: left_x, base_y: INV_TAB_Y });
        self.page_entities.push(PlacedEntity { ent: right, base_x: right_x, base_y: INV_TAB_Y });
    }

    // Убираем закладки и кнопки страниц из ECS
    fn hide_tabs(&mut self, ecs: &mut EcsAdapter) {
        let removed: Vec<Entity> = self
            .tab_entities
            .drain(..)
            .chain(self.page_entities.drain(..))
            .map(|p| p.ent)
            .collect();
        ecs.delete_entities(&removed);
    }
}
