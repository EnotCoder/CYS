use crate::scene::scene_trait::{Scene, SceneAction};
use crate::constants::*;

pub struct MenuScene {
    ready: bool,
    play_bg: Option<specs::Entity>,
    play_label: Option<specs::Entity>,
    quit_bg: Option<specs::Entity>,
    quit_label: Option<specs::Entity>,
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

    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);
        ecs.add_ui_sized(LOGO_X, LOGO_Y, LOGO_W, LOGO_H, "tex/game_name.png", device, queue);
        let pb = ecs.add_ui_sized(BTN_X, BTN_Y, BTN_W, BTN_H, "tex/black.png", device, queue);
        let qb = ecs.add_ui_sized(QUIT_X, QUIT_Y, QUIT_W, QUIT_H, "tex/black.png", device, queue);
        let pl = text_renderer.add_text(ecs, device, queue, "Play", FONT_SIZE_BTN, BTN_X, BTN_Y + 0.05, BTN_W * 0.75, 1.0, BTN_TEXT_COLOR);
        let ql = text_renderer.add_text(ecs, device, queue, "Quit", FONT_SIZE_BTN, QUIT_X, QUIT_Y + 0.05, QUIT_W * 0.75, 1.0, BTN_TEXT_COLOR);
        self.play_bg = Some(pb);
        self.play_label = Some(pl);
        self.quit_bg = Some(qb);
        self.quit_label = Some(ql);
    }

    fn is_inside(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        let Some((mx, my)) = input.cursor() else { return false };
        let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, MENU_MAP_SIZE, 0.0, 0.0);
        wx >= bx - bw / 2.0 && wx <= bx + bw / 2.0
            && wy >= by - bh / 2.0 && wy <= by + bh / 2.0
    }

    fn is_btn_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        input.mouse_pressed(MOUSE_BUTTON_LEFT) && Self::is_inside(input, window_size, bx, by, bw, bh)
    }

    fn is_play_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H)
    }

    fn is_quit_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H)
    }

    fn set_label_texture(ecs: &mut crate::EcsAdapter, label: Option<specs::Entity>, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, x: f32, y: f32, w: f32, h: f32, color: [u8; 3]) -> Option<specs::Entity> {
        if let Some(old) = label {
            ecs.delete_entity(old);
        }
        Some(text_renderer.add_text(ecs, device, queue, text, FONT_SIZE_BTN, x, y, w, h, color))
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.ready = false;
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.ready {
            self.ready = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        let h = Self::is_inside(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H);
        if h != self.play_hover {
            self.play_hover = h;
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.play_label = Self::set_label_texture(ecs, self.play_label, text_renderer, device, queue, "Play", BTN_X, BTN_Y + 0.05, BTN_W * 0.75, 1.0, color);
        }

        let h = Self::is_inside(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H);
        if h != self.quit_hover {
            self.quit_hover = h;
            let color = if h { GREEN } else { BTN_TEXT_COLOR };
            self.quit_label = Self::set_label_texture(ecs, self.quit_label, text_renderer, device, queue, "Quit", QUIT_X, QUIT_Y + 0.05, QUIT_W * 0.75, 1.0, color);
        }

        if input.key_pressed(winit::keyboard::KeyCode::Space) || Self::is_play_clicked(input, window_size) {
            return SceneAction::Switch("game".to_string());
        }
        if input.key_pressed(winit::keyboard::KeyCode::Escape) || Self::is_quit_clicked(input, window_size) {
            return SceneAction::Quit;
        }
        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        let r = ecs.get_sprites_by_layer();
        (r.0, r.1, r.2, r.3, r.4, r.5)
    }

    fn map_size(&self) -> f32 { MENU_MAP_SIZE }
    fn camera_offset(&self) -> (f32, f32) { (0.0, 0.0) }
}