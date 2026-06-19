use crate::scene::{Scene, SceneAction};
use specs::{WorldExt, Builder};

pub struct MenuScene {
    ready: bool,
    text_entity: Option<specs::Entity>,
}

impl MenuScene {
    pub fn new() -> Self {
        MenuScene { ready: false, text_entity: None }
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.ready = false;
        self.text_entity = None;

        ecs.world
            .create_entity()
            .with(crate::Transform {
                position: [0.0, 0.0, 0.0],
            })
            .with(crate::SpriteComponent {
                texture_path: "tex/menu.png".to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build();
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, _window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.ready {
            self.ready = true;

            let tex = crate::Texture::from_path(device, queue, "tex/menu.png", "menu");
            let sprite = crate::Sprite::from_texture(device, &tex, "menu", 12.0, 12.0);
            ecs.sprite_cache.insert("map_0_0_tex/menu.png_[0, 0]_[1, 1]".to_string(), sprite);

            let entity = text_renderer.add_text(
                ecs, device, queue,
                "Press space to play", 48.0, 0.0, -2.0, 3.0, 2.0, [200, 200, 200],
            );
            self.text_entity = Some(entity);
        }

        if input.key_pressed(winit::keyboard::KeyCode::Space) {
            return SceneAction::Switch("game".to_string());
        }

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 { 0.8 }
}
