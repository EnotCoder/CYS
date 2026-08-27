// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use std::io::{BufRead, BufReader};
use crate::scenes::scene_trait::{Scene, SceneAction};
use crate::core::constants::*;
use crate::ui::{Panel, create_panel, destroy_panel};
use crate::input::platform::InputSource;
use crate::save::{create_world_with_name, delete_world, list_worlds, WorldSelection, SELECTED_WORLD};
use crate::ui::text_input::TEXT_INPUT;

// ========================================================================
//  MenuScene — главное меню игры
// ========================================================================
//  Состоит из двух состояний:
//  - Main: карта-фон, логотип, кнопки Play/Quit.
//  - Worlds: выбор мира (как в Minecraft) — «Новый мир» + список сохранённых
//    миров + «Назад». При выборе мира выставляется глобальная SELECTED_WORLD,
//    игра загружает/создаёт нужный мир.

#[derive(Clone, Copy)]
enum MenuState {
    Main,
    Worlds,
    /// Ввод названия нового мира (поле + кнопки «Создать»/«Назад»)
    Naming,
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
    // Сущности кнопок Play и Quit (панель-подложка + текстовая надпись)
    play_bg: Option<Panel>,
    play_label: Option<specs::Entity>,
    quit_bg: Option<Panel>,
    quit_label: Option<specs::Entity>,
    // Текущее состояние наведения курсора для подсветки кнопок
    play_hover: bool,
    quit_hover: bool,
    // Плавный масштаб панелей при наведении (hover-анимация кнопок)
    play_scale: f32,
    quit_scale: f32,
    // Замер времени кадра для интерполяции по dt
    last_frame: Option<std::time::Instant>,
    // Адаптивный масштаб UI (и фона) под соотношение сторон экрана
    ui_scale: f32,
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
    name_create_bg: Option<Panel>,
    name_create_label: Option<specs::Entity>,
    name_back_bg: Option<Panel>,
    name_back_label: Option<specs::Entity>,
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
            play_scale: 1.0,
            quit_scale: 1.0,
            last_frame: None,
            ui_scale: MENU_MAP_SIZE,
            state: MenuState::Main,
            world_entries: Vec::new(),
            world_misc: Vec::new(),
            new_btn: None,
            back_btn: None,
            name_buffer: String::new(),
            naming_title: None,
            name_panel: None,
            name_text: None,
            name_text_key: None,
            name_create_bg: None,
            name_create_label: None,
            name_back_bg: None,
            name_back_label: None,
        }
    }

    /// Строит содержимое меню в зависимости от состояния (Main/Worlds).
    /// Сначала разрушает старое, чтобы пересоздание было идемпотентным.
    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.destroy_content(ecs);
        ecs.clear_world();

        match self.state {
            MenuState::Main => {
                crate::data::map::load_map_to_ecs(ecs);
                ecs.add_ui_sized(LOGO_X, LOGO_Y, LOGO_W, LOGO_H, "assets/tex/ui/game_name.png", device, queue);
                self.build_main(ecs, text_renderer, device, queue);
            }
            MenuState::Worlds => {
                // Тот же фон, что и в главном меню (карта + декор), но с
                // затемняющим оверлеем, чтобы список миров был читаемым.
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(0.0, 0.0, 100.0, 100.0, "assets/tex/dev_tools/black.png", device, queue);
                ecs.update_sprite_alpha(overlay, 0.6);
                self.build_worlds(ecs, text_renderer, device, queue);
            }
            MenuState::Naming => {
                // Тот же затемнённый фон, что и в выборе миров.
                crate::data::map::load_map_to_ecs(ecs);
                Self::place_decor(ecs);
                let overlay = ecs.add_ui_sized(0.0, 0.0, 100.0, 100.0, "assets/tex/dev_tools/black.png", device, queue);
                ecs.update_sprite_alpha(overlay, 0.6);
                self.build_naming(ecs, text_renderer, device, queue);
            }
        }
    }

    fn build_naming(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let title = text_renderer.add_text(ecs, device, queue, "World Name", FONT_SIZE_LOGO, 0.0, 3.7, 7.0, 1.0, WHITE);
        self.naming_title = Some(title);

        // Поле ввода (чёрная панель) с текущим названием поверх
        let mut np = Panel::new(0.0, 0.6, 6.0, 1.0, 0.5);
        create_panel(ecs, device, queue, &mut np);
        self.name_panel = Some(np);
        let nt = text_renderer.add_text(ecs, device, queue, &self.name_buffer, FONT_SIZE_BTN, 0.0, 0.65, 5.6, 1.0, WHITE);
        self.name_text = Some(nt);

        // Кнопка «Создать»
        let mut cb = Panel::new(0.0, -1.0, 3.2, 0.8, 0.5);
        create_panel(ecs, device, queue, &mut cb);
        self.name_create_bg = Some(cb);
        let cl = text_renderer.add_text(ecs, device, queue, "Create", FONT_SIZE_BTN, 0.0, -0.95, 2.4, 1.0, BTN_TEXT_COLOR);
        self.name_create_label = Some(cl);

        // Кнопка «Назад»
        let mut bb = Panel::new(0.0, -2.2, 3.2, 0.8, 0.5);
        create_panel(ecs, device, queue, &mut bb);
        self.name_back_bg = Some(bb);
        let bl = text_renderer.add_text(ecs, device, queue, "Back", FONT_SIZE_BTN, 0.0, -2.15, 2.4, 1.0, BTN_TEXT_COLOR);
        self.name_back_label = Some(bl);
    }

    fn build_main(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
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
        Self::place_decor(ecs);
    }

    fn build_worlds(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let title = text_renderer.add_text(ecs, device, queue, "Select World", FONT_SIZE_LOGO, 0.0, 3.7, 7.0, 1.0, WHITE);
        self.world_misc.push(title);

        // Кнопка «Новый мир»
        let mut nb = Panel::new(0.0, 2.4, 3.2, 0.8, 0.5);
        create_panel(ecs, device, queue, &mut nb);
        let nbl = text_renderer.add_text(ecs, device, queue, "New World", FONT_SIZE_BTN, 0.0, 2.45, 2.4, 1.0, BTN_TEXT_COLOR);
        if let Some(e) = nb.entity { self.world_misc.push(e); }
        self.world_misc.push(nbl);
        self.new_btn = Some(nb);

        // Список сохранённых миров (до 8, от недавно обновлённых)
        let worlds = list_worlds();
        let mut y = 1.2;
        for w in worlds.iter().take(8) {
            let mut pb = Panel::new(0.0, y, 3.2, 0.8, 0.5);
            create_panel(ecs, device, queue, &mut pb);
            let lbl = text_renderer.add_text(ecs, device, queue, &w.name, FONT_SIZE_BTN, 0.0, y + 0.05, 2.8, 1.0, BTN_TEXT_COLOR);
            // Кнопка удаления: чёрная панель (del_bg) с иконкой «минус» поверх
            let dx = pb.x + pb.w / 2.0 + 0.55;
            let mut db = Panel::new(dx, y, 0.7, 0.7, 0.5);
            create_panel(ecs, device, queue, &mut db);
            let di = ecs.add_ui_sized(dx, y, 0.45, 0.45, "assets/tex/ui/mini_icons/minus.png", device, queue);
            self.world_entries.push(WorldEntry { id: w.id, bg: pb, label: Some(lbl), del_bg: db, del_icon: Some(di) });
            y -= 1.0;
        }

        // Кнопка «Назад»
        let mut bb = Panel::new(0.0, -3.2, 3.2, 0.8, 0.5);
        create_panel(ecs, device, queue, &mut bb);
        let bbl = text_renderer.add_text(ecs, device, queue, "Back", FONT_SIZE_BTN, 0.0, -3.15, 2.4, 1.0, BTN_TEXT_COLOR);
        if let Some(e) = bb.entity { self.world_misc.push(e); }
        self.world_misc.push(bbl);
        self.back_btn = Some(bb);
    }

    /// Удаляет из мира все сущности меню (главного и выбора миров)
    fn destroy_content(&mut self, ecs: &mut crate::EcsAdapter) {
        if let Some(mut p) = self.play_bg.take() { destroy_panel(ecs, &mut p); }
        if let Some(mut p) = self.quit_bg.take() { destroy_panel(ecs, &mut p); }
        if let Some(e) = self.play_label.take() { ecs.delete_entity(e); }
        if let Some(e) = self.quit_label.take() { ecs.delete_entity(e); }
        for e in self.world_misc.drain(..) { ecs.delete_entity(e); }
        for mut we in self.world_entries.drain(..) {
            destroy_panel(ecs, &mut we.bg);
            if let Some(l) = we.label.take() { ecs.delete_entity(l); }
            destroy_panel(ecs, &mut we.del_bg);
            if let Some(l) = we.del_icon.take() { ecs.delete_entity(l); }
        }
        if let Some(e) = self.naming_title.take() { ecs.delete_entity(e); }
        if let Some(mut p) = self.name_panel.take() { destroy_panel(ecs, &mut p); }
        if let Some(e) = self.name_text.take() { ecs.delete_entity(e); }
        self.name_text_key = None;
        if let Some(mut p) = self.name_create_bg.take() { destroy_panel(ecs, &mut p); }
        if let Some(e) = self.name_create_label.take() { ecs.delete_entity(e); }
        if let Some(mut p) = self.name_back_bg.take() { destroy_panel(ecs, &mut p); }
        if let Some(e) = self.name_back_label.take() { ecs.delete_entity(e); }
        self.new_btn = None;
        self.back_btn = None;
    }

    /// Расставляет декоративные объекты магазина по файлу menu_shop.txt.
    /// Каждый токен-буква соответствует предмету (b — box, r — rack, c — cassa и т.д.).
    fn place_decor(ecs: &mut crate::EcsAdapter) {
        let bytes = crate::core::asset::load_bytes("assets/menu_shop.txt").expect("menu_shop.txt not found!");
        let reader = BufReader::new(&bytes[..]);
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
                    false,
                    slot.obj.animated,
                    slot.obj.frame_paths,
                );
            }
        }
        ecs.update_fence_textures();
    }

    /// Проверка, находится ли курсор мыши над прямоугольником кнопки
    fn is_inside(input: &dyn InputSource, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        let Some((mx, my)) = input.cursor() else { return false };
        let (wx, wy) = crate::ui::system::ndc_to_ui(mx, my, window_size);
        wx >= bx - bw / 2.0 && wx <= bx + bw / 2.0
            && wy >= by - bh / 2.0 && wy <= by + bh / 2.0
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
        // При каждом возврате в меню начинаем с главного экрана
        self.state = MenuState::Main;
        self.ready = false;
        crate::audio::play_music("music");
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &dyn InputSource, window_size: (f32, f32), text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        // Замер dt кадра (используется для hover-анимации в update_main)
        let _dt = match self.last_frame {
            Some(t0) => t0.elapsed().as_secs_f64(),
            None => 1.0 / 60.0,
        };
        self.last_frame = Some(std::time::Instant::now());

        // Адаптивный масштаб UI
        let aspect = if window_size.1 > 0.0 { window_size.0 / window_size.1 } else { 1.0 };
        self.ui_scale = crate::core::util::ui_fit_scale(aspect, 3.6);

        if !self.ready {
            self.ready = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        match self.state {
            MenuState::Main => self.update_main(input, window_size, ecs, text_renderer, device, queue),
            MenuState::Worlds => self.update_worlds(input, window_size, ecs, text_renderer, device, queue),
            MenuState::Naming => self.update_naming(input, window_size, ecs, text_renderer, device, queue),
        }
    }

    fn sprites(&self, ecs: &crate::EcsAdapter, _visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer(None)
    }

    fn map_size(&self) -> f32 { self.ui_scale }
    fn ui_size(&self) -> f32 { self.ui_scale }
    fn camera_offset(&self) -> (f32, f32) { (0.0, 0.0) }
}

impl MenuScene {
    /// Логика главного экрана: подсветка Play/Quit и обработка кликов.
    fn update_main(&mut self, input: &dyn InputSource, window_size: (f32, f32), ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        // Подсветка кнопки Play при наведении (зелёный текст вместо обычного)
        let h = Self::is_inside(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H);
        if h != self.play_hover {
            self.play_hover = h;
            if h { crate::audio::play("hover"); }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.play_label = Self::set_label_texture(ecs, self.play_label, text_renderer, device, queue, "Play", BTN_X, BTN_Y + 0.05, BTN_W * 0.75, 1.0, color);
        }

        // Подсветка кнопки Quit при наведении
        let h = Self::is_inside(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H);
        if h != self.quit_hover {
            self.quit_hover = h;
            if h { crate::audio::play("hover"); }
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.quit_label = Self::set_label_texture(ecs, self.quit_label, text_renderer, device, queue, "Quit", QUIT_X, QUIT_Y + 0.05, QUIT_W * 0.75, 1.0, color);
        }

        // Hover-масштаб кнопок: плавно нарастает/возвращается (~за 5 кадров)
        let k = 0.2;
        let play_target = if self.play_hover { 1.12 } else { 1.0 };
        let quit_target = if self.quit_hover { 1.12 } else { 1.0 };
        self.play_scale += (play_target - self.play_scale) * k;
        self.quit_scale += (quit_target - self.quit_scale) * k;
        if (play_target - self.play_scale).abs() < 0.0001 { self.play_scale = play_target; }
        if (quit_target - self.quit_scale).abs() < 0.0001 { self.quit_scale = quit_target; }
        if let Some(bg) = &self.play_bg {
            if let Some(e) = bg.entity { ecs.update_sprite_scale(e, self.play_scale); }
        }
        if let Some(bg) = &self.quit_bg {
            if let Some(e) = bg.entity { ecs.update_sprite_scale(e, self.quit_scale); }
        }

        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left) && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Запуск игры по пробелу или клику на Play
        if input.key_pressed(winit::keyboard::KeyCode::Space) || clicked(input, BTN_X, BTN_Y, BTN_W, BTN_H) {
            crate::audio::play("click");
            self.state = MenuState::Worlds;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }
        // Выход по Escape или клику на Quit
        if input.key_pressed(winit::keyboard::KeyCode::Escape) || clicked(input, QUIT_X, QUIT_Y, QUIT_W, QUIT_H) {
            crate::audio::play("click");
            return SceneAction::Quit;
        }
        SceneAction::None
    }

    /// Логика выбора миров: «Новый мир», список миров, «Назад».
    fn update_worlds(&mut self, input: &dyn InputSource, window_size: (f32, f32), ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left) && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Новый мир — переходим в состояние ввода названия
        if let Some(ref nb) = self.new_btn {
            if clicked(input, nb.x, nb.y, nb.w, nb.h) {
                crate::audio::play("click");
                self.name_buffer = "Мир".to_string();
                TEXT_INPUT.set_active(true);
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
            delete_world(id);
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

    /// Логика ввода названия: захват символов из TEXT_INPUT, отрисовка поля,
    /// кнопки «Создать» (подтверждение) и «Назад».
    fn update_naming(&mut self, input: &dyn InputSource, window_size: (f32, f32), ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        let clicked = |input: &dyn InputSource, bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            input.mouse_pressed(winit::event::MouseButton::Left) && Self::is_inside(input, window_size, bx, by, bw, bh)
        };

        // Забираем накопленный ввод: обычные символы, Backspace ('\u{8}'),
        // Enter ('\n') — последний подтверждает создание.
        let typed = TEXT_INPUT.take();
        for ch in typed.chars() {
            match ch {
                '\u{8}' => { self.name_buffer.pop(); }
                '\n' => { return self.confirm_naming(ecs, text_renderer, device, queue); }
                c if !c.is_control() => {
                    if self.name_buffer.chars().count() < 20 {
                        self.name_buffer.push(c);
                    }
                }
                _ => {}
            }
        }

        // Отражаем текущее название в поле (текстура меняется только при изменении)
        let (e, k) = text_renderer.set_text(ecs, device, queue, self.name_text, self.name_text_key, &self.name_buffer, FONT_SIZE_BTN, 0.0, 0.65, 5.6, 1.0, WHITE);
        self.name_text = e;
        self.name_text_key = k;

        // Escape — назад к списку миров
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            TEXT_INPUT.set_active(false);
            self.state = MenuState::Worlds;
            self.setup_content(ecs, text_renderer, device, queue);
            return SceneAction::None;
        }

        // Создать
        if let Some(ref cb) = self.name_create_bg {
            if clicked(input, cb.x, cb.y, cb.w, cb.h) {
                crate::audio::play("click");
                return self.confirm_naming(ecs, text_renderer, device, queue);
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
        SceneAction::None
    }

    /// Подтверждает ввод: создаёт мир с уникальным именем и запускает игру.
    fn confirm_naming(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer, _device: &wgpu::Device, _queue: &wgpu::Queue) -> SceneAction {
        let name = self.name_buffer.trim();
        let name = if name.is_empty() { "Мир" } else { name };
        let meta = create_world_with_name(name);
        TEXT_INPUT.set_active(false);
        *SELECTED_WORLD.lock().unwrap() = WorldSelection::New(meta.id, meta.name);
        SceneAction::Switch("game".to_string())
    }
}
