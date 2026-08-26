// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use std::collections::HashMap;
use specs::WorldExt;
use crate::scenes::scene_trait::Scene;
use crate::core::constants::*;

// ========================================================================
//  SceneManager — реестр сцен и переключение между ними
// ========================================================================
//  Хранит единый мир EcsAdapter и все зарегистрированные сцены.
//  При переключении сцены мир полностью очищается (все сущности, кэш
//  спрайтов, панели/курсоры), затем вызывается on_enter новой сцены,
//  поэтому каждая сцена строит свои дочерние ресурсы с нуля.

pub struct SceneManager {
    pub ecs: crate::EcsAdapter,
    pub scenes: HashMap<String, Box<dyn Scene>>,
    pub current: String,
    // Ресурсы счётчика FPS: пересоздаются из кэша при изменении значения
    fps_entity: Option<specs::Entity>,
    fps_sprite_key: Option<u64>,
    last_fps: u32,
    last_clear_count: u64,
}

impl SceneManager {
    pub fn new() -> Self {
        let ecs = crate::EcsAdapter::new();
        let mut scenes: HashMap<String, Box<dyn Scene>> = HashMap::new();

        // Регистрируем доступные сцены; стартуем с главного меню
        scenes.insert("menu".to_string(), Box::new(crate::scenes::MenuScene::new()));
        scenes.insert("game".to_string(), Box::new(crate::scenes::GameScene::new()));

        SceneManager {
            ecs,
            scenes,
            current: "menu".to_string(),
            fps_entity: None,
            fps_sprite_key: None,
            last_fps: 0,
            last_clear_count: 0,
        }
    }

    /// Переключает активную сцену: останавливает музыку, очищает мир,
    /// вызывает on_enter новой сцены
    pub fn switch_to(&mut self, name: &str, text_renderer: &mut crate::ui::text_renderer::TextRenderer) {
        crate::audio::stop_music();
        self.clear_ecs_world();
        if let Some(scene) = self.scenes.get_mut(name) {
            scene.on_enter(&mut self.ecs, text_renderer);
            self.current = name.to_string();
        }
    }

    /// Обновляет строку FPS в углу экрана только когда значение изменилось,
    /// чтобы не создавать сущность на каждом кадре
    pub fn update_fps(
        &mut self,
        fps: u32,
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let cleared = self.last_clear_count != self.ecs.clear_count;
        if !cleared && fps == self.last_fps {
            return;
        }
        self.last_clear_count = self.ecs.clear_count;
        self.last_fps = fps;

        let fps_text = format!("FPS: {}", fps);

        // Удаляем старые спрайт и сущность перед вставкой новых
        if let Some(entity) = self.fps_entity.take() {
            self.ecs.delete_entity(entity);
        }
        if let Some(key) = self.fps_sprite_key.take() {
            self.ecs.sprite_cache.remove(&key);
        }

        let entity = text_renderer.add_text(
            &mut self.ecs, device, queue,
            &fps_text, 64.0, 7.2, 4.0, 1.0, 4.0, WHITE,
        );

        self.fps_entity = Some(entity);
        self.fps_sprite_key = Some(crate::ui::text_renderer::TextRenderer::sprite_cache_key(
            &fps_text, 24.0, 1.0, GREEN,
        ));
    }

    /// Полностью очищает мир ECS: удаляет все сущности с Transform и Sprite,
    /// сбрасывает кэши, группы, курсор-превью и ID групп
    fn clear_ecs_world(&mut self) {
        use specs::Join;
        // Собираем всех сущностей, у которых есть и Transform, и Sprite
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

        // Сбрасываем FPS-счётчик, т.к. мир сцен теперь пуст
        self.fps_entity = None;
        self.fps_sprite_key = None;
        self.last_fps = 0;
    }
}
