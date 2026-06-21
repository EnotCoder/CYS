use crate::scene::scene_trait::{Scene, SceneAction};
use crate::constants::SHADER_SCALE;

const BTN_X: f32 = 0.0;
const BTN_Y: f32 = -0.5;
const BTN_W: f32 = 2.0;
const BTN_H: f32 = 0.8;
const QUIT_X: f32 = 0.0;
const QUIT_Y: f32 = -1.5;
const QUIT_W: f32 = 2.0;
const QUIT_H: f32 = 0.8;
const MENU_MAP_SIZE: f32 = 0.8;

pub struct MenuScene {
    ready: bool,
}

impl MenuScene {
    pub fn new() -> Self {
        MenuScene { ready: false }
    }

    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);
        ecs.add_ui_sized(0.0, 2.0, 2.5, 2.5, "tex/game_name.png", device, queue);
        ecs.add_button(device, queue, BTN_X, BTN_Y, BTN_W, BTN_H, "Play", 48.0, text_renderer);
        ecs.add_button(device, queue, QUIT_X, QUIT_Y, QUIT_W, QUIT_H, "Quit", 48.0, text_renderer);
    }

    fn is_btn_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), bx: f32, by: f32, bw: f32, bh: f32) -> bool {
        if !input.mouse_pressed(0) {
            return false;
        }
        let Some((mx, my)) = input.cursor() else { return false };
        let scale = SHADER_SCALE * MENU_MAP_SIZE;
        let wx = ((mx / window_size.0) * 2.0 - 1.0) / scale;
        let wy = (1.0 - (my / window_size.1) * 2.0) / scale;
        wx >= bx - bw / 2.0 && wx <= bx + bw / 2.0
            && wy >= by - bh / 2.0 && wy <= by + bh / 2.0
    }

    fn is_play_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, BTN_X, BTN_Y, BTN_W, BTN_H)
    }

    fn is_quit_clicked(input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) -> bool {
        Self::is_btn_clicked(input, window_size, QUIT_X, QUIT_Y, QUIT_W, QUIT_H)
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

        if input.key_pressed(winit::keyboard::KeyCode::Space) || Self::is_play_clicked(input, window_size) {
            return SceneAction::Switch("game".to_string());
        }
        if input.key_pressed(winit::keyboard::KeyCode::Escape) || Self::is_quit_clicked(input, window_size) {
            return SceneAction::Quit;
        }
        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 { MENU_MAP_SIZE }
}
