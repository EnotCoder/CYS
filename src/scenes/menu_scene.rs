// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use crate::core::constants::*;
use crate::input::platform::InputSource;
use crate::save::{
    create_world_with_name, delete_world, list_worlds, WorldSelection, SELECTED_WORLD,
};
use crate::scenes::scene_trait::{Scene, SceneAction};
use crate::ui::mods::Mods;
use crate::ui::settings::Settings;
use crate::ui::text_input::TEXT_INPUT;
use crate::ui::{create_button, create_panel, destroy_button, destroy_panel, Button, Panel};
use std::io::{BufRead, BufReader};

use crate::scenes::game::weather_fx;
use specs::WorldExt;

const MENU_APPEAR_DURATION: f32 = 0.45;
const MENU_APPEAR_STAGGER: f32 = 0.09;

// ========================================================================
//  MenuScene — главное меню игры
// ========================================================================
//  Состоит из состояний:
//  - Main: карта-фон, логотип, кнопки Play/Settings/Mods/Quit.
//  - Settings: те же настройки, что в игре, но без кнопки «В меню».
//  - Mods: панель включения/выключения модов с прокруткой (Escape/Close — назад).
//  - Worlds: выбор мира (как в Minecraft) — «Новый мир» + список сохранённых
//    миров + «Назад». При выборе мира выставляется глобальная SELECTED_WORLD,
//    игра загружает/создаёт нужный мир.

#[derive(Clone, Copy)]
enum MenuState {
    /// Сплэш-экран: логотип assets/icon.png при запуске приложения
    Splash,
    Main,
    /// Настройки (без кнопки возврата в меню — мы уже в нём)
    Settings,
    /// Панель Mods: список модов с галочками и прокруткой
    Mods,
    Worlds,
    /// Ввод названия нового мира (поле + кнопки «Создать»/«Назад»)
    Naming,
    ConfirmDelete(u32),
}

/// Запись списка миров: id + панель-кнопка + текст-надпись + кнопка удаления (X)
struct WorldEntry {
    id: u32,
    bg: Panel,
    label: Option<specs::Entity>,
    del_bg: Panel,
    del_icon: Option<specs::Entity>,
}

pub struct MenuScene {
    ready: bool,
    // Сплэш-экран показывается один раз при самом первом запуске
    splash_shown: bool,
    splash_timer: f64,
    splash_bg: Option<specs::Entity>,
    splash_logo: Option<specs::Entity>,
    // Логотип главного меню и время для синусоидального покачивания
    logo: Option<specs::Entity>,
    logo_time: f32,
    // Сущности кнопок Play, Settings, Mods и Quit (подложка + текст)
    play_bg: Option<Panel>,
    play_label: Option<specs::Entity>,
    settings_bg: Option<Panel>,
    settings_label: Option<specs::Entity>,
    mods_bg: Option<Panel>,
    mods_label: Option<specs::Entity>,
    quit_bg: Option<Panel>,
    quit_label: Option<specs::Entity>,
    // Текущее состояние наведения курсора для подсветки кнопок
    play_hover: bool,
    settings_hover: bool,
    mods_hover: bool,
    quit_hover: bool,
    // Плавный масштаб панелей при наведении (hover-анимация кнопок)
    play_scale: f32,
    settings_scale: f32,
    mods_scale: f32,
    quit_scale: f32,
    // Окна настроек и модов (используются и в игре)
    settings_ui: Settings,
    mods_ui: Mods,
    // Замер времени кадра для интерполяции по dt
    last_frame: Option<std::time::Instant>,
    // Адаптивный масштаб фона под соотношение сторон экрана
    ui_scale: f32,
    // Масштаб UI-слоя: в подменю шире, чтобы вместить панели до 10.0 x 7.4
    ui_size_scale: f32,
    // Состояние меню: главное / выбор миров / ввод названия
    state: MenuState,
    // UI выбора миров: кнопки миров и прочие сущности (для очистки)
    world_entries: Vec<WorldEntry>,
    world_misc: Vec<specs::Entity>,
    new_btn: Option<Panel>,
    back_btn: Option<Panel>,
    // Состояние ввода названия мира
    name_buffer: String,
    naming_title: Option<specs::Entity>,
    name_panel: Option<Panel>,
    name_text: Option<specs::Entity>,
    name_text_key: Option<u64>,
    cursor_timer: f32,
    name_create_bg: Option<Panel>,
    name_create_label: Option<specs::Entity>,
    name_back_bg: Option<Panel>,
    name_back_label: Option<specs::Entity>,
    delete_confirm_panel: Option<Panel>,
    delete_confirm_delete_btn: Option<Button>,
    delete_confirm_cancel_btn: Option<Button>,
    // Плавное появление логотипа и кнопок после входа в меню
    appear_time: f32,
    // Погода
    weather_fx: weather_fx::WeatherFx,
}

impl MenuScene {
    pub fn new() -> Self {
        MenuScene {
            ready: false,
            splash_shown: false,
            splash_timer: 0.0,
            splash_bg: None,
            splash_logo: None,
            logo: None,
            logo_time: 0.0,
            play_bg: None,
            play_label: None,
            settings_bg: None,
            settings_label: None,
            mods_bg: None,
            mods_label: None,
            quit_bg: None,
            quit_label: None,
            play_hover: false,
            settings_hover: false,
            mods_hover: false,
            quit_hover: false,
            play_scale: 1.0,
            settings_scale: 1.0,
            mods_scale: 1.0,
            quit_scale: 1.0,
            settings_ui: {
                let mut ui = Settings::new();
                ui.hide_menu_button();
                ui
            },
            mods_ui: Mods::new(),
            last_frame: None,
            ui_scale: MENU_MAP_SIZE,
            ui_size_scale: MENU_MAP_SIZE,
            // Стартуем со сплэш-экрана (on_enter при самом первом запуске не вызывается)
            state: MenuState::Splash,
            world_entries: Vec::new(),
            world_misc: Vec::new(),
            new_btn: None,
            back_btn: None,
            name_buffer: String::new(),
            naming_title: None,
            name_panel: None,
            name_text: None,
            name_text_key: None,
            cursor_timer: 0.0,
            name_create_bg: None,
            name_create_label: None,
            name_back_bg: None,
            name_back_label: None,
            delete_confirm_panel: None,
            delete_confirm_delete_btn: None,
            delete_confirm_cancel_btn: None,
            appear_time: 0.0,
            weather_fx: weather_fx::WeatherFx::new(),
        }
    }

    /// Строит содержимое меню в зависимости от состояния (Main/Worlds).
    /// Сначала разрушает старое, чтобы пересоздание было идемпотентным.
    fn setup_content(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.destroy_content(ecs);
        ecs.clear_world();
        self.weather_fx.reset();

        match self.state {
            MenuState::Splash => {
                // Чёрный фон на весь экран для контраста логотипа
                let bg = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    400.0,
                    400.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                self.splash_bg = Some(bg);
                // Логотип пользователя (иконка приложения)
                let logo = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    SPLASH_LOGO_W,
                    SPLASH_LOGO_H,
                    "assets/icon.png",
                    device,
                    queue,
                );
                self.splash_logo = Some(logo);
                self.splash_timer = 0.0;
            }
            MenuState::Main => {
                crate::data::map::load_map_to_ecs(ecs);
                self.appear_time = 0.0;
                self.logo = Some(ecs.add_ui_sized(
                    LOGO_X,
                    LOGO_Y,
                    LOGO_W,
                    LOGO_H,
                    "assets/tex/ui/game_name.png",
                    device,
                    queue,
                ));
                self.logo_time = 0.0;
                self.build_main(ecs, text_renderer, device, queue);
            }
            MenuState::Settings => {
                // Тот же затемнённый фон, что и в других подменю
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                ecs.update_sprite_alpha(overlay, 0.6);
                {
                    let saved = crate::save::SETTINGS.lock().unwrap().clone();
                    self.settings_ui.apply_saved(&saved);
                }
                self.settings_ui.open(ecs, text_renderer, device, queue);
            }
            MenuState::Mods => {
                // Тот же затемнённый фон, что и в других подменю
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                ecs.update_sprite_alpha(overlay, 0.6);
                self.mods_ui.open(ecs, text_renderer, device, queue);
            }
            MenuState::Worlds => {
                // Тот же фон, что и в главном меню (карта + декор), но с
                // затемняющим оверлеем, чтобы список миров был читаемым.
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                ecs.update_sprite_alpha(overlay, 0.6);
                self.build_worlds(ecs, text_renderer, device, queue);
            }
            MenuState::Naming => {
                // Тот же затемнённый фон, что и в выборе миров.
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                ecs.update_sprite_alpha(overlay, 0.6);
                self.build_naming(ecs, text_renderer, device, queue);
            }
            MenuState::ConfirmDelete(_) => {
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                    "assets/tex/dev_tools/black.png",
                    device,
                    queue,
                );
                ecs.update_sprite_alpha(overlay, 0.6);
                self.build_worlds(ecs, text_renderer, device, queue);
                self.build_delete_confirm(ecs, text_renderer, device, queue);
            }
        }
    }

    fn build_naming(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let title = text_renderer.add_text(
            ecs,
            device,
            queue,
            "World Name",
            FONT_SIZE_LOGO,
            0.0,
            3.7,
            7.0,
            1.0,
            WHITE,
        );
        self.naming_title = Some(title);

        // Поле ввода (чёрная панель) с текущим названием поверх
        let mut np = Panel::new(0.0, 0.6, 6.0, 1.0, 0.5);
        create_panel(ecs, device, queue, &mut np);
        self.name_panel = Some(np);
        let nt = text_renderer.add_text(
            ecs,
            device,
            queue,
            &self.name_buffer,
            FONT_SIZE_BTN,
            0.0,
            0.65,
            5.6,
            1.0,
            WHITE,
        );
        self.name_text = Some(nt);
        self.cursor_timer = 0.0;

        // Кнопка «Создать»
        let mut cb = Panel::textured(0.0, -1.0, 3.2, 0.8, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut cb);
        self.name_create_bg = Some(cb);
        let cl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Create",
            FONT_SIZE_BTN,
            0.0,
            -0.95,
            2.4,
            1.0,
            BTN_TEXT_COLOR,
        );
        self.name_create_label = Some(cl);

        // Кнопка «Назад»
        let mut bb = Panel::textured(0.0, -2.2, 3.2, 0.8, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut bb);
        self.name_back_bg = Some(bb);
        let bl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Back",
            FONT_SIZE_BTN,
            0.0,
            -2.15,
            2.4,
            1.0,
            BTN_TEXT_COLOR,
        );
        self.name_back_label = Some(bl);
    }

    fn build_main(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut play_panel = Panel::textured(BTN_X, BTN_Y, BTN_W, BTN_H, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut play_panel);
        let mut settings_panel = Panel::textured(
            MENU_SETTINGS_X,
            MENU_SETTINGS_Y,
            MENU_SETTINGS_W,
            MENU_SETTINGS_H,
            1.0,
            TEX_BUTTON,
        );
        create_panel(ecs, device, queue, &mut settings_panel);
        let mut mods_panel = Panel::textured(MODS_X, MODS_Y, MODS_W, MODS_H, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut mods_panel);
        let mut quit_panel = Panel::textured(QUIT_X, QUIT_Y, QUIT_W, QUIT_H, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut quit_panel);
        let pl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Play",
            FONT_SIZE_BTN,
            BTN_X,
            BTN_Y + 0.05,
            BTN_W * 0.75,
            1.0,
            BTN_TEXT_COLOR,
        );
        let sl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Settings",
            FONT_SIZE_BTN,
            MENU_SETTINGS_X,
            MENU_SETTINGS_Y + 0.05,
            MENU_SETTINGS_W * 0.75,
            1.0,
            BTN_TEXT_COLOR,
        );
        let ml = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Mods",
            FONT_SIZE_BTN,
            MODS_X,
            MODS_Y + 0.05,
            MODS_W * 0.75,
            1.0,
            BTN_TEXT_COLOR,
        );
        let ql = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Quit",
            FONT_SIZE_BTN,
            QUIT_X,
            QUIT_Y + 0.05,
            QUIT_W * 0.75,
            1.0,
            BTN_TEXT_COLOR,
        );
        self.play_bg = Some(play_panel);
        self.play_label = Some(pl);
        self.settings_bg = Some(settings_panel);
        self.settings_label = Some(sl);
        self.mods_bg = Some(mods_panel);
        self.mods_label = Some(ml);
        self.quit_bg = Some(quit_panel);
        self.quit_label = Some(ql);
        Self::place_decor(ecs);
    }

    fn build_worlds(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let title = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Select World",
            FONT_SIZE_LOGO,
            0.0,
            3.7,
            7.0,
            1.0,
            WHITE,
        );
        self.world_misc.push(title);

        // Кнопка «Новый мир»
        let mut nb = Panel::textured(0.0, 2.4, 3.2, 0.8, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut nb);
        let nbl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "New World",
            FONT_SIZE_BTN,
            0.0,
            2.45,
            2.4,
            1.0,
            BTN_TEXT_COLOR,
        );
        if let Some(e) = nb.entity {
            self.world_misc.push(e);
        }
        self.world_misc.push(nbl);
        self.new_btn = Some(nb);

        // Список сохранённых миров (до 8, от недавно обновлённых)
        let worlds = list_worlds();
        let mut y = 1.2;
        for w in worlds.iter().take(8) {
            let mut pb = Panel::textured(0.0, y, 3.2, 0.8, 1.0, TEX_BUTTON);
            create_panel(ecs, device, queue, &mut pb);
            let lbl = text_renderer.add_text(
                ecs,
                device,
                queue,
                &w.name,
                FONT_SIZE_BTN,
                0.0,
                y + 0.05,
                2.8,
                1.0,
                BTN_TEXT_COLOR,
            );
            // Кнопка удаления: чёрная панель (del_bg) с иконкой «минус» поверх
            let dx = pb.x + pb.w / 2.0 + 0.55;
            let mut db = Panel::new(dx, y, 0.7, 0.7, 0.5);
            create_panel(ecs, device, queue, &mut db);
            let di = ecs.add_ui_sized(
                dx,
                y,
                0.45,
                0.45,
                "assets/tex/ui/mini_icons/minus.png",
                device,
                queue,
            );
            self.world_entries.push(WorldEntry {
                id: w.id,
                bg: pb,
                label: Some(lbl),
                del_bg: db,
                del_icon: Some(di),
            });
            y -= 1.0;
        }

        // Кнопка «Назад»
        let mut bb = Panel::textured(0.0, -3.2, 3.2, 0.8, 1.0, TEX_BUTTON);
        create_panel(ecs, device, queue, &mut bb);
        let bbl = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Back",
            FONT_SIZE_BTN,
            0.0,
            -3.15,
            2.4,
            1.0,
            BTN_TEXT_COLOR,
        );
        if let Some(e) = bb.entity {
            self.world_misc.push(e);
        }
        self.world_misc.push(bbl);
        self.back_btn = Some(bb);
    }

    fn build_delete_confirm(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let overlay = ecs.add_ui_sized(
            0.0,
            0.0,
            100.0,
            100.0,
            "assets/tex/dev_tools/black.png",
            device,
            queue,
        );
        ecs.update_sprite_alpha(overlay, 0.55);
        self.world_misc.push(overlay);

        let mut panel = Panel::new(0.0, 0.0, 6.0, 3.0, 0.9);
        create_panel(ecs, device, queue, &mut panel);
        self.delete_confirm_panel = Some(panel);

        let message = text_renderer.add_text(
            ecs,
            device,
            queue,
            "Are you sure?",
            FONT_SIZE_BTN,
            0.0,
            0.65,
            5.2,
            1.0,
            WHITE,
        );
        self.world_misc.push(message);

        let mut delete = Button::new(-1.2, -0.8, 2.0, 0.8, "Delete");
        create_button(ecs, text_renderer, device, queue, &mut delete);
        self.delete_confirm_delete_btn = Some(delete);

        let mut cancel = Button::new(1.2, -0.8, 2.0, 0.8, "Cancel");
        create_button(ecs, text_renderer, device, queue, &mut cancel);
        self.delete_confirm_cancel_btn = Some(cancel);
    }

    fn destroy_content(&mut self, ecs: &mut crate::EcsAdapter) {
        if let Some(e) = self.splash_bg.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.splash_logo.take() {
            ecs.delete_entity(e);
        }
        self.logo = None;
        self.settings_ui.reset(ecs);
        self.mods_ui.reset(ecs);
        if let Some(mut p) = self.play_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(mut p) = self.settings_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(mut p) = self.mods_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(mut p) = self.quit_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(e) = self.play_label.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.settings_label.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.mods_label.take() {
            ecs.delete_entity(e);
        }
        if let Some(e) = self.quit_label.take() {
            ecs.delete_entity(e);
        }
        for e in self.world_misc.drain(..) {
            ecs.delete_entity(e);
        }
        for mut we in self.world_entries.drain(..) {
            destroy_panel(ecs, &mut we.bg);
            if let Some(l) = we.label.take() {
                ecs.delete_entity(l);
            }
            destroy_panel(ecs, &mut we.del_bg);
            if let Some(l) = we.del_icon.take() {
                ecs.delete_entity(l);
            }
        }
        if let Some(e) = self.naming_title.take() {
            ecs.delete_entity(e);
        }
        if let Some(mut p) = self.name_panel.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(e) = self.name_text.take() {
            ecs.delete_entity(e);
        }
        self.name_text_key = None;
        if let Some(mut p) = self.name_create_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(e) = self.name_create_label.take() {
            ecs.delete_entity(e);
        }
        if let Some(mut p) = self.name_back_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(e) = self.name_back_label.take() {
            ecs.delete_entity(e);
        }
        if let Some(mut p) = self.delete_confirm_panel.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(mut b) = self.delete_confirm_delete_btn.take() {
            destroy_button(ecs, &mut b);
        }
        if let Some(mut b) = self.delete_confirm_cancel_btn.take() {
            destroy_button(ecs, &mut b);
        }
        self.new_btn = None;
        self.back_btn = None;
    }

    /// Расставляет декоративные объекты магазина по файлу menu_shop.txt.
    /// Каждый токен-буква соответствует предмету (b — box, r — rack, c — cassa и т.д.).
    pub(crate) fn place_decor(ecs: &mut crate::EcsAdapter) {
        let bytes = crate::core::asset::load_bytes("assets/menu_shop.txt")
            .expect("menu_shop.txt not found!");
        let reader = BufReader::new(&bytes[..]);
        for (j, line) in reader.lines().flatten().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            for (i, token) in parts.iter().enumerate() {
                // Токен "." — пустая клетка, пропускаем её
                let name = match *token {
                    "." => continue,
                    "b" => "box",
                    "s" => "sign",
                    "r" => "rack",
                    "t" => "table",
                    "c" => "cassa",
                    "i" => "ice_cream",
                    "d" => "candies",
                    "a" => "arcade_machine",
                    "f" => "fence",
                    "w" => "welcome",
                    "0" => "blue_carpet",
                    "1" => "red_carpet",
                    "2" => "green_carpet",
                    "3" => "white_carpet",
                    "4" => "black_carpet",
                    "5" => "iron_panel",
                    "6" => "gold_panel",
                    "7" => "diamond_panel",
                    _ => continue,
                };
                let x = i as f32 + WORLD_OFFSET_X;
                let y = -(j as f32) + WORLD_OFFSET_Y;
                let slot = crate::data::make_slot(name);
                // Добавляем объект как группу, чтобы проработал размер и текстуры
                ecs.add_group_object(
                    x as i32,
                    y as i32,
                    slot.obj.width,
                    slot.obj.height,
                    name,
                    slot.obj.path,
                    slot.obj.texture_frame,
                    slot.obj.texture_count,
                    crate::data::is_carpet_name(name),
                    false,
                    slot.obj.animated,
                    slot.obj.frame_paths,
                );
            }
        }
        ecs.update_fence_textures();
    }

    /// Alpha, смещение сверху и масштаб элемента с задержкой по очереди.
    fn appear_state(elapsed: f32, index: usize) -> (f32, f32, f32) {
        let delay = index as f32 * MENU_APPEAR_STAGGER;
        let t = ((elapsed - delay) / MENU_APPEAR_DURATION).clamp(0.0, 1.0);
        let alpha = 1.0 - (1.0 - t).powi(3);
        let drop = 0.35 * (1.0 - alpha);
        let scale = 0.92 + 0.08 * crate::core::util::ease_out_back(t);
        (alpha, drop, scale)
    }

    fn is_inside(
        input: &dyn InputSource,
        window_size: (f32, f32),
        bx: f32,
        by: f32,
        bw: f32,
        bh: f32,
    ) -> bool {
        let Some((mx, my)) = input.cursor() else {
            return false;
        };
        let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
        wx >= bx - bw / 2.0 && wx <= bx + bw / 2.0 && wy >= by - bh / 2.0 && wy <= by + bh / 2.0
    }

    fn set_label_texture(
        ecs: &mut crate::EcsAdapter,
        label: Option<specs::Entity>,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [u8; 3],
    ) -> Option<specs::Entity> {
        // Удаляем старую надпись и создаём новую с другим цветом
        if let Some(old) = label {
            ecs.delete_entity(old);
        }
        Some(text_renderer.add_text(ecs, device, queue, text, FONT_SIZE_BTN, x, y, w, h, color))
    }
}

impl Scene for MenuScene {
    fn on_enter(
        &mut self,
        _ecs: &mut crate::EcsAdapter,
        _text_renderer: &mut crate::ui::text_renderer::TextRenderer,
    ) {
        // При самом первом запуске открываем сплэш-экран, дальше сразу в главное меню
        self.state = if self.splash_shown {
            MenuState::Main
        } else {
            MenuState::Splash
        };
        self.ready = false;
        self.weather_fx.reset();
        crate::audio::play_music("music");
    }

    fn update(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        input: &dyn InputSource,
        window_size: (f32, f32),
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        // Замер dt кадра (используется для hover-анимации в update_main)
        let dt = match self.last_frame {
            Some(t0) => t0.elapsed().as_secs_f64(),
            None => 1.0 / 60.0,
        }
        .min(MAX_FRAME_DT);

        self.last_frame = Some(std::time::Instant::now());

        // Адаптивный масштаб UI
        let aspect = if window_size.1 > 0.0 {
            window_size.0 / window_size.1
        } else {
            1.0
        };
        self.ui_scale = crate::core::util::ui_fit_scale(aspect, MENU_UI_HALF_W, MENU_UI_HALF_H);
        // Панели Settings (9.0 x 6.0) и Mods (10.0 x 7.4) шире обычного меню,
        // поэтому в этих состояниях масштаб UI считается по «игровым» границам.
        // Фон остаётся на месте (map_size по-прежнему берёт self.ui_scale).
        let wide_ui = matches!(self.state, MenuState::Settings | MenuState::Mods);
        let (ui_half_w, ui_half_h) = if wide_ui {
            (GAME_UI_HALF_W, GAME_UI_HALF_H)
        } else {
            (MENU_UI_HALF_W, MENU_UI_HALF_H)
        };
        self.ui_size_scale = crate::core::util::ui_fit_scale(aspect, ui_half_w, ui_half_h);

        if !self.ready {
            self.ready = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        self.weather_fx.restart_ambient();

        let season = *ecs.world.read_resource::<crate::ecs::components::Season>();
        self.weather_fx.tick(ecs, device, queue, season, dt);

        match self.state {
            MenuState::Splash => self.update_splash(dt, input, ecs, text_renderer, device, queue),
            MenuState::Main => {
                self.update_main(dt, input, window_size, ecs, text_renderer, device, queue)
            }
            MenuState::Settings => {
                self.update_settings(dt, input, window_size, ecs, text_renderer, device, queue)
            }
            MenuState::Mods => {
                self.update_mods(dt, input, window_size, ecs, text_renderer, device, queue)
            }
            MenuState::Worlds => {
                self.update_worlds(input, window_size, ecs, text_renderer, device, queue)
            }
            MenuState::Naming => {
                self.update_naming(dt, input, window_size, ecs, text_renderer, device, queue)
            }
            MenuState::ConfirmDelete(id) => self.update_delete_confirm(
                id,
                input,
                window_size,
                ecs,
                text_renderer,
                device,
                queue,
            ),
        }
    }

    fn sprites(
        &self,
        ecs: &crate::EcsAdapter,
        visible_bounds: Option<(f32, f32, f32, f32)>,
    ) -> (
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
    ) {
        // Камера меню неподвижна, поэтому отсечение по видимой области так же
        // безопасно, как в игре. Фон карты — один sized-спрайт, он отсекается
        // по своему размеру, а не по центру (см. get_sprites_by_layer).
        ecs.get_sprites_by_layer(visible_bounds)
    }

    fn map_size(&self) -> f32 {
        self.ui_scale
    }
    fn ui_size(&self) -> f32 {
        self.ui_size_scale
    }
    fn camera_offset(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
}

impl MenuScene {
    /// Сплэш-экран: лого показывается «попом» (fade-in + scale upward),
    /// затем мягко «дышит» (медленная пульсация масштаба на удержании),
    /// потом затухает — и переходим в главное меню. Клик/клавиша скипают.
    fn update_splash(
        &mut self,
        dt: f64,
        input: &dyn InputSource,
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        self.splash_timer += dt;

        // Пропуск сплэша любым кликом или клавишей действия
        if input.mouse_pressed(winit::event::MouseButton::Left)
            || input.key_pressed(winit::keyboard::KeyCode::Space)
            || input.key_pressed(winit::keyboard::KeyCode::Enter)
            || input.key_pressed(winit::keyboard::KeyCode::Escape)
        {
            return self.finish_splash(ecs, text_renderer, device, queue);
        }

        let t = self.splash_timer;
        let fade_in = SPLASH_FADE_IN;
        let hold = SPLASH_FADE_IN + SPLASH_HOLD;
        let total = SPLASH_FADE_IN + SPLASH_HOLD + SPLASH_FADE_OUT;

        // Вычисляем alpha и scale лого по фазам анимации
        let (alpha, scale): (f32, f32) = if t < fade_in {
            let p = (t / fade_in).clamp(0.0, 1.0) as f32;
            let smooth = p * p * (3.0 - 2.0 * p);
            let e = crate::core::util::ease_out_back(p);
            (smooth, 0.7 + 0.3 * e)
        } else if t < hold {
            // Дыхание: мягкая пульсация масштаба на весь период удержания
            let p = ((t - fade_in) / SPLASH_HOLD).clamp(0.0, 1.0) as f32;
            (1.0, 1.0 + 0.05 * (p * std::f32::consts::TAU).sin())
        } else {
            let p = ((t - hold) / SPLASH_FADE_OUT).clamp(0.0, 1.0) as f32;
            let smooth = p * p * (3.0 - 2.0 * p);
            (1.0 - smooth, 1.0 + 0.08 * p)
        };

        if let Some(e) = self.splash_logo {
            ecs.update_sprite_alpha(e, alpha);
            ecs.update_sprite_scale(e, scale);
        }

        if t >= total {
            return self.finish_splash(ecs, text_renderer, device, queue);
        }
        SceneAction::None
    }

    /// Завершает сплэш и передаёт управление вступительной кат-сцене.
    /// Возврат в меню после кат-сцены уже не показывает ни сплэш, ни интро.
    fn finish_splash(
        &mut self,
        _ecs: &mut crate::EcsAdapter,
        _text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> SceneAction {
        self.splash_shown = true;
        self.state = MenuState::Main;
        SceneAction::Switch("intro".to_string())
    }

    /// Логика главного экрана: подсветка Play/Settings/Mods/Quit и клики.
    fn update_main(
        &mut self,
        dt: f64,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        // Синусоидальное покачивание логотипа по вертикали (dt клампим, чтобы
        // после фриза/перетаскивания окна логотип не прыгал)
        let frame_dt = dt.clamp(0.0, 0.1) as f32;
        self.logo_time += frame_dt;
        self.appear_time += frame_dt;
        let logo_offset = MENU_LOGO_AMPLITUDE * (self.logo_time * MENU_LOGO_SPEED).sin();
        let (logo_alpha, logo_drop, logo_scale) = Self::appear_state(self.appear_time, 0);
        if let Some(e) = self.logo {
            ecs.update_transform_position(e, LOGO_X, LOGO_Y + logo_offset + logo_drop);
            ecs.update_sprite_alpha(e, logo_alpha);
            ecs.update_sprite_scale(e, logo_scale);
        }

        // Подсветка кнопки Play при наведении (зелёный текст вместо обычного)
        let h = Self::is_inside(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H);
        if h != self.play_hover {
            self.play_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.play_label = Self::set_label_texture(
                ecs,
                self.play_label,
                text_renderer,
                device,
                queue,
                "Play",
                BTN_X,
                BTN_Y + 0.05,
                BTN_W * 0.75,
                1.0,
                color,
            );
        }

        // Подсветка кнопки Settings при наведении
        let h = Self::is_inside(
            input,
            window_size,
            MENU_SETTINGS_X,
            MENU_SETTINGS_Y,
            MENU_SETTINGS_W,
            MENU_SETTINGS_H,
        );
        if h != self.settings_hover {
            self.settings_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.settings_label = Self::set_label_texture(
                ecs,
                self.settings_label,
                text_renderer,
                device,
                queue,
                "Settings",
                MENU_SETTINGS_X,
                MENU_SETTINGS_Y + 0.05,
                MENU_SETTINGS_W * 0.75,
                1.0,
                color,
            );
        }

        // Подсветка кнопки Mods при наведении
        let h = Self::is_inside(input, window_size, MODS_X, MODS_Y, MODS_W, MODS_H);
        if h != self.mods_hover {
            self.mods_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.mods_label = Self::set_label_texture(
                ecs,
                self.mods_label,
                text_renderer,
                device,
                queue,
                "Mods",
                MODS_X,
                MODS_Y + 0.05,
                MODS_W * 0.75,
                1.0,
                color,
            );
        }

        // Подсветка кнопки Quit при наведении
        let h = Self::is_inside(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H);
        if h != self.quit_hover {
            self.quit_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.quit_label = Self::set_label_texture(
                ecs,
                self.quit_label,
                text_renderer,
                device,
                queue,
                "Quit",
                QUIT_X,
                QUIT_Y + 0.05,
                QUIT_W * 0.75,
                1.0,
                color,
            );
        }

        // Hover-масштаб кнопок: плавно нарастает/возвращается (~за 5 кадров)
        let k = 0.2;
        let play_target = if self.play_hover { 1.12 } else { 1.0 };
        let settings_target = if self.settings_hover { 1.12 } else { 1.0 };
        let mods_target = if self.mods_hover { 1.12 } else { 1.0 };
        let quit_target = if self.quit_hover { 1.12 } else { 1.0 };
        self.play_scale += (play_target - self.play_scale) * k;
        self.settings_scale += (settings_target - self.settings_scale) * k;
        self.mods_scale += (mods_target - self.mods_scale) * k;
        self.quit_scale += (quit_target - self.quit_scale) * k;
        if (play_target - self.play_scale).abs() < 0.0001 {
            self.play_scale = play_target;
        }
        if (settings_target - self.settings_scale).abs() < 0.0001 {
            self.settings_scale = settings_target;
        }
        if (mods_target - self.mods_scale).abs() < 0.0001 {
            self.mods_scale = mods_target;
        }
        if (quit_target - self.quit_scale).abs() < 0.0001 {
            self.quit_scale = quit_target;
        }
        let buttons = [
            (
                self.play_bg.as_ref().and_then(|p| p.entity),
                self.play_label,
                BTN_X,
                BTN_Y,
                self.play_scale,
                1,
            ),
            (
                self.settings_bg.as_ref().and_then(|p| p.entity),
                self.settings_label,
                MENU_SETTINGS_X,
                MENU_SETTINGS_Y,
                self.settings_scale,
                2,
            ),
            (
                self.mods_bg.as_ref().and_then(|p| p.entity),
                self.mods_label,
                MODS_X,
                MODS_Y,
                self.mods_scale,
                3,
            ),
            (
                self.quit_bg.as_ref().and_then(|p| p.entity),
                self.quit_label,
                QUIT_X,
                QUIT_Y,
                self.quit_scale,
                4,
            ),
        ];
        for (bg, label, x, y, hover_scale, index) in buttons {
            let (alpha, drop, scale) = Self::appear_state(self.appear_time, index);
            if let Some(e) = bg {
                ecs.update_transform_position(e, x, y + drop);
                ecs.update_sprite_alpha(e, alpha);
                ecs.update_sprite_scale(e, hover_scale * scale);
            }
            if let Some(e) = label {
                // Масштаб текстового спрайта менять нельзя: он входит в ключ
                // sprite_cache, а виртуальный путь текста не является файлом —
                // пока scale != 1.0, рендер показал бы null.png.
                ecs.update_transform_position(e, x, y + 0.05 + drop);
                ecs.update_sprite_alpha(e, alpha);
            }
        }

        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left)
                && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Запуск игры по пробелу или клику на Play
        if input.key_pressed(winit::keyboard::KeyCode::Space)
            || clicked(input, BTN_X, BTN_Y, BTN_W, BTN_H)
        {
            crate::audio::play("click");
            self.state = MenuState::Worlds;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        // Настройки по клику на Settings
        if clicked(
            input,
            MENU_SETTINGS_X,
            MENU_SETTINGS_Y,
            MENU_SETTINGS_W,
            MENU_SETTINGS_H,
        ) {
            crate::audio::play("click");
            self.state = MenuState::Settings;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        // Панель модов по клику на Mods
        if clicked(input, MODS_X, MODS_Y, MODS_W, MODS_H) {
            crate::audio::play("click");
            self.state = MenuState::Mods;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        // Выход по Escape или клику на Quit
        if input.key_pressed(winit::keyboard::KeyCode::Escape)
            || clicked(input, QUIT_X, QUIT_Y, QUIT_W, QUIT_H)
        {
            crate::audio::play("click");
            return SceneAction::Quit;
        }
        SceneAction::None
    }

    /// Записывает текущие значения панели в data/settings.json.
    fn persist_settings(&self) {
        let s = crate::save::GameSettings {
            vsync: self.settings_ui.vsync.checked,
            zoom_speed: self.settings_ui.zoom_speed.value,
            music: self.settings_ui.music.checked,
            sfx: self.settings_ui.sfx.checked,
            texture_pack: self.settings_ui.texture_pack_id(),
            shader: self.settings_ui.shader_id(),
            mods: crate::scripts::mods::enabled_ids(),
        };
        *crate::save::SETTINGS.lock().unwrap() = s.clone();
        crate::save::save_settings(&s);
    }

    /// Настройки в меню — те же элементы, что в игре, но без кнопки «В меню».
    /// Escape / клик вне панели возвращают в главное меню.
    fn update_settings(
        &mut self,
        dt: f64,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        self.settings_ui.tick(ecs, dt);
        self.settings_ui.tick_hover(ecs, input, window_size, dt);

        let mut outside_click = false;
        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
            let p = &self.settings_ui.panel;
            let inside = (wx - p.x).abs() <= p.w / 2.0 && (wy - p.y).abs() <= p.h / 2.0;
            outside_click = input.mouse_pressed(winit::event::MouseButton::Left) && !inside;
        }
        self.settings_ui
            .handle_input(ecs, text_renderer, device, queue, input, window_size);

        // Кнопка «Mods...» внутри настроек открывает тот же список модов
        if self.settings_ui.mods_requested {
            self.settings_ui.mods_requested = false;
            self.state = MenuState::Mods;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        if self.settings_ui.vsync_toggled {
            self.settings_ui.vsync_toggled = false;
            let enabled = self.settings_ui.vsync.checked;
            self.persist_settings();
            return SceneAction::VsyncToggle(enabled);
        }
        if self.settings_ui.zoom_speed_changed {
            self.settings_ui.zoom_speed_changed = false;
            self.persist_settings();
        }
        if self.settings_ui.music_toggled {
            self.settings_ui.music_toggled = false;
            crate::audio::set_music_enabled(self.settings_ui.music.checked);
            if self.settings_ui.music.checked {
                crate::audio::play_music("music");
            }
            self.persist_settings();
        }
        if self.settings_ui.sfx_toggled {
            self.settings_ui.sfx_toggled = false;
            crate::audio::set_sfx_enabled(self.settings_ui.sfx.checked);
            self.persist_settings();
        }
        if self.settings_ui.texture_pack_changed {
            self.settings_ui.texture_pack_changed = false;
            let id = self.settings_ui.texture_pack_id();
            crate::core::texture_pack::set_active(id.as_deref());
            ecs.apply_texture_pack();
            self.persist_settings();
        }
        if self.settings_ui.shader_changed {
            self.settings_ui.shader_changed = false;
            let id = self.settings_ui.shader_id();
            crate::core::shader_pack::set_active(id.as_deref());
            self.persist_settings();
            return SceneAction::ShaderChanged;
        }

        if input.key_pressed(winit::keyboard::KeyCode::Escape) || outside_click {
            crate::audio::play("click");
            self.state = MenuState::Main;
            self.setup_content(ecs, text_renderer, device, queue);
        }
        SceneAction::None
    }

    /// Панель Mods в меню: галочки, прокрутка, возврат по Escape/Close/вне окна.
    fn update_mods(
        &mut self,
        dt: f64,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        self.mods_ui.tick(ecs, dt);
        self.mods_ui.tick_hover(ecs, input, window_size, dt);

        // Клик вне панели тоже возвращает в главное меню
        let mut outside_click = false;
        if let Some((mx, my)) = input.cursor() {
            let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
            let p = &self.mods_ui.panel;
            let inside = (wx - p.x).abs() <= p.w / 2.0 && (wy - p.y).abs() <= p.h / 2.0;
            outside_click = input.mouse_pressed(winit::event::MouseButton::Left) && !inside;
        }

        self.mods_ui
            .handle_input(ecs, text_renderer, device, queue, input, window_size);

        if self.mods_ui.changed {
            self.mods_ui.changed = false;
            crate::scripts::mods::persist_enabled(&self.mods_ui.enabled_ids());
            ecs.apply_texture_pack();
            self.mods_ui
                .refresh_statuses(ecs, text_renderer, device, queue);
        }

        // Escape, клик вне окна или нажатие Close закрывают панель
        if input.key_pressed(winit::keyboard::KeyCode::Escape)
            || outside_click
            || !self.mods_ui.open
        {
            crate::audio::play("click");
            self.state = MenuState::Main;
            self.setup_content(ecs, text_renderer, device, queue);
        }
        SceneAction::None
    }

    /// Логика выбора миров: «Новый мир», список миров, «Назад».
    fn update_worlds(
        &mut self,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left)
                && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Новый мир — переходим в состояние ввода названия
        if let Some(ref nb) = self.new_btn {
            if clicked(input, nb.x, nb.y, nb.w, nb.h) {
                crate::audio::play("click");
                self.name_buffer = "World".to_string();
                TEXT_INPUT.request_focus_with_text(&self.name_buffer);
                self.state = MenuState::Naming;
                self.setup_content(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
        }
        // Список миров (удаление или загрузка)
        let mut del_id = None;
        for we in &self.world_entries {
            if clicked(input, we.del_bg.x, we.del_bg.y, we.del_bg.w, we.del_bg.h) {
                del_id = Some(we.id);
                break;
            }
            if clicked(input, we.bg.x, we.bg.y, we.bg.w, we.bg.h) {
                crate::audio::play("click");
                *SELECTED_WORLD.lock().unwrap() = WorldSelection::Load(we.id);
                return SceneAction::Switch("game".to_string());
            }
        }
        if let Some(id) = del_id {
            crate::audio::play("click");
            self.state = MenuState::ConfirmDelete(id);
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        // Назад
        if let Some(ref bb) = self.back_btn {
            if clicked(input, bb.x, bb.y, bb.w, bb.h) {
                crate::audio::play("click");
                self.state = MenuState::Main;
                self.setup_content(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
        }
        SceneAction::None
    }

    fn update_delete_confirm(
        &mut self,
        id: u32,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        if input.key_pressed(winit::keyboard::KeyCode::Escape)
            || input.key_pressed(winit::keyboard::KeyCode::BrowserBack)
        {
            crate::audio::play("click");
            self.state = MenuState::Worlds;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }

        if let Some(button) = &self.delete_confirm_delete_btn {
            if input.key_pressed(winit::keyboard::KeyCode::Enter)
                || crate::ui::button_clicked(button, input, window_size)
            {
                crate::audio::play("click");
                delete_world(id);
                self.state = MenuState::Worlds;
                self.setup_content(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
        }

        if let Some(button) = &self.delete_confirm_cancel_btn {
            if crate::ui::button_clicked(button, input, window_size) {
                crate::audio::play("click");
                self.state = MenuState::Worlds;
                self.setup_content(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
        }
        SceneAction::None
    }

    /// Логика ввода названия: захват символов из TEXT_INPUT, отрисовка поля,
    /// кнопки «Создать» (подтверждение) и «Назад».
    fn update_naming(
        &mut self,
        dt: f64,
        input: &dyn InputSource,
        window_size: (f32, f32),
        ecs: &mut crate::EcsAdapter,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SceneAction {
        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left)
                && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Забираем накопленный ввод: обычные символы, Backspace ('\u{8}'),
        // Enter ('\n') — последний подтверждает создание.
        #[allow(unused_mut)]
        let mut typed = TEXT_INPUT.take();
        let mut enter = false;
        #[cfg(target_os = "android")]
        if let Some((text, native_enter)) = crate::android::text_input_value() {
            let original_len = text.chars().count();
            self.name_buffer = text
                .chars()
                .filter(|ch| !ch.is_control())
                .take(20)
                .collect();
            if original_len > 20 {
                crate::android::set_text_input_value(&self.name_buffer);
            }
            enter = native_enter;
            typed.clear();
            TEXT_INPUT.clear_preedit();
        }
        for ch in typed.chars() {
            match ch {
                '\u{8}' => {
                    self.name_buffer.pop();
                }
                '\n' => {
                    enter = true;
                }
                c if !c.is_control() => {
                    if self.name_buffer.chars().count() < 20 {
                        self.name_buffer.push(c);
                    }
                }
                _ => {}
            }
        }
        // Текущая IME-композиция: на телефоне визуальная клавиатура держит
        // набираемый текст здесь и часто не шлёт отдельного Commit, поэтому
        // его тоже учитываем в итоговом названии.
        let pre = TEXT_INPUT.preedit();
        let display = format!("{}{}", self.name_buffer, pre);
        self.cursor_timer = (self.cursor_timer + dt as f32) % 1.0;
        let display_with_cursor = if TEXT_INPUT.is_active() && self.cursor_timer < 0.5 {
            format!("{}|", display)
        } else {
            display.clone()
        };

        // Отражаем текущее название в поле (текстура меняется только при изменении)
        let (e, k) = text_renderer.set_text(
            ecs,
            device,
            queue,
            self.name_text,
            self.name_text_key,
            &display_with_cursor,
            FONT_SIZE_BTN,
            0.0,
            0.65,
            5.6,
            1.0,
            WHITE,
        );
        self.name_text = e;
        self.name_text_key = k;

        // Escape — назад к списку миров
        if input.key_pressed(winit::keyboard::KeyCode::Escape)
            || input.key_pressed(winit::keyboard::KeyCode::BrowserBack)
        {
            TEXT_INPUT.set_active(false);
            self.state = MenuState::Worlds;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }

        // Создать (Enter на клавиатуре)
        if enter {
            return self.confirm_naming(ecs, text_renderer, device, queue, &display);
        }
        // Создать (кнопка)
        if let Some(ref cb) = self.name_create_bg {
            if clicked(input, cb.x, cb.y, cb.w, cb.h) {
                crate::audio::play("click");
                return self.confirm_naming(ecs, text_renderer, device, queue, &display);
            }
        }
        // Назад
        if let Some(ref bb) = self.name_back_bg {
            if clicked(input, bb.x, bb.y, bb.w, bb.h) {
                crate::audio::play("click");
                TEXT_INPUT.set_active(false);
                self.state = MenuState::Worlds;
                self.setup_content(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
        }

        if input.mouse_pressed(winit::event::MouseButton::Left) {
            let in_name_field = self
                .name_panel
                .as_ref()
                .map(|panel| {
                    Self::is_inside(input, window_size, panel.x, panel.y, panel.w, panel.h)
                })
                .unwrap_or(false);
            if in_name_field {
                self.cursor_timer = 0.0;
                TEXT_INPUT.request_focus_with_text(&self.name_buffer);
            } else {
                TEXT_INPUT.set_active(false);
            }
        }
        SceneAction::None
    }

    /// Подтверждает ввод: создаёт мир с уникальным именем и запускает игру.
    fn confirm_naming(
        &mut self,
        _ecs: &mut crate::EcsAdapter,
        _text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        name: &str,
    ) -> SceneAction {
        let name = name.trim();
        let name = if name.is_empty() { "World" } else { name };
        let meta = create_world_with_name(name);
        TEXT_INPUT.set_active(false);
        *SELECTED_WORLD.lock().unwrap() = WorldSelection::New(meta.id);
        SceneAction::Switch("game".to_string())
    }
}
