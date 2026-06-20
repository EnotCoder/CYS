pub mod menu_scene;
pub mod game_scene;

use std::collections::HashMap;
use specs::WorldExt;

pub enum SceneAction {
    Switch(String),
    Quit,
    None,
}

pub trait Scene {
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer);
    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction;
    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>);
    fn map_size(&self) -> f32;
}

pub struct SceneManager {
    pub ecs: crate::EcsAdapter,
    pub scenes: HashMap<String, Box<dyn Scene>>,
    pub current: String,
    fps_entity: Option<specs::Entity>,
    fps_sprite_key: Option<String>,
    current_fps_text: String,
}

impl SceneManager {
    pub fn new(text_renderer: &mut crate::text_renderer::TextRenderer) -> Self {
        let ecs = crate::EcsAdapter::new();
        let mut scenes: HashMap<String, Box<dyn Scene>> = HashMap::new();

        let menu = menu_scene::MenuScene::new();
        scenes.insert("menu".to_string(), Box::new(menu));

        let game = game_scene::GameScene::new();
        scenes.insert("game".to_string(), Box::new(game));

        let mut sm = SceneManager {
            ecs,
            scenes,
            current: "menu".to_string(),
            fps_entity: None,
            fps_sprite_key: None,
            current_fps_text: String::new(),
        };
        if let Some(scene) = sm.scenes.get_mut(&sm.current) {
            scene.on_enter(&mut sm.ecs, text_renderer);
        }
        sm
    }

    pub fn switch_to(&mut self, name: &str, text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.clear_ecs_world();
        if let Some(scene) = self.scenes.get_mut(name) {
            scene.on_enter(&mut self.ecs, text_renderer);
            self.current = name.to_string();
        }
    }

    pub fn update_fps(
        &mut self,
        fps: u32,
        text_renderer: &mut crate::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let fps_text = format!("FPS: {}", fps);

        if fps_text == self.current_fps_text {
            if let Some(entity) = self.fps_entity {
                if self.ecs.world.entities().is_alive(entity) {
                    return;
                }
            }
        }

        if let Some(entity) = self.fps_entity.take() {
            let _ = self.ecs.world.entities().delete(entity);
            self.ecs.world.write_storage::<crate::Transform>().remove(entity);
            self.ecs.world.write_storage::<crate::SpriteComponent>().remove(entity);
        }
        if let Some(key) = self.fps_sprite_key.take() {
            self.ecs.sprite_cache.remove(&key);
        }

        let entity = text_renderer.add_text(
            &mut self.ecs, device, queue,
            &fps_text, 24.0, 3.5, 3.8, 1.5, 1.0, [0, 255, 0],
        );

        self.fps_entity = Some(entity);
        self.fps_sprite_key = Some(crate::text_renderer::TextRenderer::sprite_cache_key(
            3.5, 3.8, &fps_text, 24.0, 1.0, [0, 255, 0],
        ));
        self.current_fps_text = fps_text;
    }

    fn clear_ecs_world(&mut self) {
        use specs::Join;
        let delete_entities: Vec<specs::Entity> = {
            let entities = self.ecs.world.entities();
            let transforms = self.ecs.world.read_storage::<crate::Transform>();
            let sprites = self.ecs.world.read_storage::<crate::SpriteComponent>();
            (&entities, &transforms, &sprites).join()
                .map(|(e, _, _)| e)
                .collect()
        };

        {
            let mut transforms = self.ecs.world.write_storage::<crate::Transform>();
            let mut sprites = self.ecs.world.write_storage::<crate::SpriteComponent>();
            let entities = self.ecs.world.entities();
            for e in delete_entities {
                transforms.remove(e);
                sprites.remove(e);
                let _ = entities.delete(e);
            }
        }

        self.ecs.sprite_cache.clear();
        self.ecs.cursor_preview.clear();
        self.ecs.next_group_id = 1;
        self.ecs.world.write_resource::<crate::GroupInfoResource>().groups.clear();
    }
}
