use crate::scene::{Scene, SceneAction};

pub struct MenuScene {
    ready: bool,
}

impl MenuScene {
    pub fn new() -> Self {
        MenuScene { ready: false }
    }

    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);

        //name 
        ecs.add_ui_sized(0.0, 2.0, 2.5, 2.5, "tex/game_name.png", device, queue);

        //black_panel
        ecs.add_ui_sized(0.0, -1.0, 2.5, 2.5, "tex/black.png", device, queue);

        //Play && Quit
        text_renderer.add_text(
            ecs, device, queue,
            "Space to play", 48.0, 0.0, -0.5, 2.0, 2.0, [200, 200, 200],
        );

        text_renderer.add_text(
            ecs, device, queue,
            "Esc to quit", 48.0, 0.0, -1.5, 2.0, 2.0, [200, 200, 200],
        );
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.ready = false;
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, _window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.ready {
            self.ready = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        if input.key_pressed(winit::keyboard::KeyCode::Space) {
            return SceneAction::Switch("game".to_string());
        }
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            return SceneAction::Quit;
        }

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 { 0.8 }
}
