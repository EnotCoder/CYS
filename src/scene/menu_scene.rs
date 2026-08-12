// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::constants::*;
use crate::ui::{Panel, create_panel, destroy_panel};

// ========================================================================
//  MenuScene — главное меню игры
// ========================================================================
//  Содержит карту-фон, логотип, кнопки Play/Quit и декоративную расстановку
//  предметов (читается из menu_shop.txt). Контент создаётся один раз при
//  первом входе (флаг ready), при повторных входах — очищается и строится заново.

pub struct MenuScene {
    ready: bool,
    // Сущности кнопок Play и Quit (панель-подложка + текстовая надпись)
    play_bg: Option<Panel>,
    play_label: Option<specs::Entity>,
    quit_bg: Option<Panel>,
    quit_label: Option<specs::Entity>,
    // Текущее состояние наведения курсора для подсветки кнопок
    play_hover: bool,
    quit_hover: bool,
}

impl MenuScene {
    pub fn new() -> Self {
        MenuScene {
            ready: false,
            play_bg: None,
            play_label: None,
            quit_bg: None,
            quit_label: None,
            play_hover: false,
            quit_hover: false,
        }
    }

    /// Строит всё содержимое меню: карта, логотип, панели и надписи кнопок, декор.
    /// Сначала разрушает старое, чтобы пересоздание было идемпотентным.
    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.destroy_content(ecs);
        crate::load_map_to_ecs(ecs);
        ecs.add_ui_sized(LOGO_X, LOGO_Y, LOGO_W, LOGO_H, "tex/ui/game_name.png", device, queue);
        // Панели-подложки кнопок Play и Quit
        let mut play_panel = Panel::new(BTN_X, BTN_Y, BTN_W, BTN_H, 0.5);
        create_panel(ecs, device, queue, &mut play_panel);
        let mut quit_panel = Panel::new(QUIT_X, QUIT_Y, QUIT_W, QUIT_H, 0.5);
        create_panel(ecs, device, queue, &mut quit_panel);
        let pl = text_renderer.add_text(ecs, device, queue, "Play", FONT_SIZE_BTN, BTN_X, BTN_Y + 0.05, BTN_W * 0.75, 1.0, BTN_TEXT_COLOR);
        let ql = text_renderer.add_text(ecs, device, queue, "Quit", FONT_SIZE_BTN, QUIT_X, QUIT_Y + 0.05, QUIT_W * 0.75, 1.0, BTN_TEXT_COLOR);
        self.play_bg = Some(play_panel);
        self.play_label = Some(pl);
        self.quit_bg = Some(quit_panel);
        self.quit_label = Some(ql);
        ecs.add_ui(LOGO_UI_X, LOGO_UI_Y, TEX_MY_LOGO);
        Self::place_decor(ecs);
    }

    /// Удаляет из мира сущности кнопок и их панелей
    fn destroy_content(&mut self, ecs: &mut crate::EcsAdapter) {
        if let Some(mut p) = self.play_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(mut p) = self.quit_bg.take() {
            destroy_panel(ecs, &mut p);
        }
        if let Some(ent) = self.play_label.take() {
            ecs.delete_entity(ent);
        }
        if let Some(ent) = self.quit_label.take() {
            ecs.delete_entity(ent);
        }
    }

    /// Расставляет декоративные объекты магазина по файлу menu_shop.txt.
    /// Каждый токен-буква соответствует предмету (b — box, r — rack, c — cassa и т.д.).
    fn place_decor(ecs: &mut crate::EcsAdapter) {
        let file = File::open("menu_shop.txt").expect("menu_shop.txt not found!");
        let reader = BufReader::new(file);
        for (j, line) in reader.lines().flatten().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
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
                    x as i32, y as i32,
                    slot.obj.width, slot.obj.height,
                    slot.obj.path,
                    slot.obj.texture_frame,
                    slot.obj.texture_count,
                    crate::data::is_carpet_name(name),
                    slot.obj.animated,
                    slot.obj.frame_paths,
                );
            }
        }
        ecs.update_fence_textures();
    }

    /// Проверка, находится ли курсор мыши над прямоугольником кнопки
    fn is_inside(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        let Some((mx, my)) = input.cursor() else { return false };
        let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, MENU_MAP_SIZE, 0.0, 0.0);
        wx >= bx - bw / 2.0 && wx <= bx + bw / 2.0
            && wy >= by - bh / 2.0 && wy <= by + bh / 2.0
    }

    /// ЛКМ зажата и курсор внутри заданной области
    fn is_btn_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        input.mouse_pressed(winit::event::MouseButton::Left) && Self::is_inside(input, window_size, bx, by, bw, bh)
    }

    fn is_play_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H)
    }

    fn is_quit_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H)
    }

    fn set_label_texture(ecs: &mut crate::EcsAdapter, label: Option<specs::Entity>, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, x: f32, y: f32, w: f32, h: f32, color: [u8; 3]) -> Option<specs::Entity> {
        // Удаляем старую надпись и создаём новую с другим цветом
        if let Some(old) = label {
            ecs.delete_entity(old);
        }
        Some(text_renderer.add_text(ecs, device, queue, text, FONT_SIZE_BTN, x, y, w, h, color))
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer) {
        // Откладываем построение контента до первого update(), а в меню играет музыка
        self.ready = false;
        crate::audio::play_music("music");
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.ready {
            self.ready = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        // Подсветка кнопки Play при наведении (зелёный текст вместо обычного)
        let h = Self::is_inside(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H);
        if h != self.play_hover {
            self.play_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.play_label = Self::set_label_texture(ecs, self.play_label, text_renderer, device, queue, "Play", BTN_X, BTN_Y + 0.05, BTN_W * 0.75, 1.0, color);
        }

        // Подсветка кнопки Quit при наведении
        let h = Self::is_inside(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H);
        if h != self.quit_hover {
            self.quit_hover = h;
            if h {
                crate::audio::play("hover");
            }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.quit_label = Self::set_label_texture(ecs, self.quit_label, text_renderer, device, queue, "Quit", QUIT_X, QUIT_Y + 0.05, QUIT_W * 0.75, 1.0, color);
        }

        // Запуск игры по пробелу или клику на Play
        if input.key_pressed(winit::keyboard::KeyCode::Space) || Self::is_play_clicked(input, window_size) {
            crate::audio::play("click");
            return SceneAction::Switch("game".to_string());
        }
        // Выход по Escape или клику на Quit
        if input.key_pressed(winit::keyboard::KeyCode::Escape) || Self::is_quit_clicked(input, window_size) {
            crate::audio::play("click");
            return SceneAction::Quit;
        }
        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter, _visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        // В меню отдаём все слои, отсечение не нужно
        let r = ecs.get_sprites_by_layer(None);
        (r.0, r.1, r.2, r.3, r.4, r.5)
    }

    fn map_size(&self) -> f32 { MENU_MAP_SIZE }
    fn camera_offset(&self) -> (f32, f32) { (0.0, 0.0) }
}
