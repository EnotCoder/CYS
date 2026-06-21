use specs::{WorldExt, Builder};
use crate::scene::{Scene, SceneAction};
use crate::text_renderer::TextRenderer;

const INV_ITEMS: &[&str] = &["box", "sign", "rack", "table", "carpet", "red_carpet", "green_carpet"];

fn slot_texture(name: &str) -> String {
    format!("tex/ui/icon_slots/{}.png", name)
}

pub struct GameScene {
    loaded: bool,
    loading: bool,
    loading_text: Option<specs::Entity>,
    loading_sprite_key: Option<String>,
    slots: Vec<crate::slot_object::Slot>,
    act_slot: i32,
    mode: i32,
    map_size: f32,
    cursor_entity: Option<specs::Entity>,
    icon_mode: Option<specs::Entity>,
    icons_slot_cursor: Option<specs::Entity>,
    inventory_open: bool,
    inventory_slots: Vec<specs::Entity>,
    inventory_mode: bool,
    inv_cursor_entity: Option<specs::Entity>,
    inv_selected: i32,
    slot_entities: Vec<specs::Entity>,
}

impl GameScene {
    pub fn new() -> Self {
        GameScene {
            loaded: false,
            loading: false,
            loading_text: None,
            loading_sprite_key: None,
            slots: Vec::new(),
            act_slot: 0,
            mode: 0,
            map_size: 1.0,
            cursor_entity: None,
            icon_mode: None,
            icons_slot_cursor: None,
            inventory_open: false,
            inventory_slots: Vec::new(),
            inventory_mode: false,
            inv_selected: 0,
            inv_cursor_entity: None,
            slot_entities: Vec::new(),
        }
    }

    fn show_loading(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let entity = text_renderer.add_text(
            ecs, device, queue,
            "Loading...", 64.0, 0.0, 0.0, 4.0, 2.0, [200, 200, 200],
        );
        self.loading_text = Some(entity);
        self.loading_sprite_key = Some(TextRenderer::sprite_cache_key(0.0, 0.0, "Loading...", 48.0, 2.0, [200, 200, 200]));
    }

    fn hide_loading(&mut self, ecs: &mut crate::EcsAdapter) {
        if let Some(entity) = self.loading_text.take() {
            let _ = ecs.world.entities().delete(entity);
            ecs.world.write_storage::<crate::Transform>().remove(entity);
            ecs.world.write_storage::<crate::SpriteComponent>().remove(entity);
        }
        if let Some(key) = self.loading_sprite_key.take() {
            ecs.sprite_cache.remove(&key);
        }
    }

    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);

        self.slots = crate::slot_object::get_slot_vec();

        text_renderer.add_text(
            ecs, device, queue,
            "Pre alpha", 128.0, -3.0, 3.8, 2.0, 4.0, [255, 255, 255],
        );

        let icon_mode = ecs.add_ui(4.0, -4.0, "tex/ui/mode/standart_mode.png");

        for (i, slot) in self.slots.iter().enumerate() {
            let ent = ecs.add_ui(
                -4.0 + i as f32,
                -4.0,
                &format!("tex/ui/icon_slots/{}.png", slot.obj.name),
            );
            self.slot_entities.push(ent);
        }

        let icons_slot_cursor = ecs.add_ui(-4.0, -4.0, "tex/ui/icon_slots/cursor.png");

        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, "tex/cursor/def_cursor.png"));
    }

    fn show_inventory(&mut self, ecs: &mut crate::EcsAdapter) {
        for row in (0..5).rev() {
            for col in 0..5 {
                let item_idx = (4 - row) * 5 + col;
                let tex = if item_idx < INV_ITEMS.len() {
                    slot_texture(INV_ITEMS[item_idx])
                } else {
                    "tex/ui/icon_slots/null.png".to_string()
                };
                let ent = ecs.add_ui(
                    -4.0 + col as f32,
                    -3.0 + row as f32,
                    &tex,
                );
                self.inventory_slots.push(ent);
            }
        }
    }

    fn hide_inventory(&mut self, ecs: &mut crate::EcsAdapter) {
        let to_remove: Vec<specs::Entity> = self.inventory_slots.drain(..).collect();
        let entities = ecs.world.entities();
        let mut transforms = ecs.world.write_storage::<crate::Transform>();
        let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
        for &ent in &to_remove {
            transforms.remove(ent);
            sprites.remove(ent);
            let _ = entities.delete(ent);
        }
    }

    fn make_cursor_entity(ecs: &mut crate::EcsAdapter, x: f32, y: f32, z: f32) -> specs::Entity {
        ecs.world.create_entity()
            .with(crate::Transform { position: [x, y, z] })
            .with(crate::SpriteComponent {
                texture_path: "tex/ui/icon_slots/cursor.png".to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
            })
            .build()
    }

    fn enter_inventory(&mut self, ecs: &mut crate::EcsAdapter) {
        self.show_inventory(ecs);
        let new_cursor = Self::make_cursor_entity(ecs, -4.0, 1.0, 3.0);
        self.inv_cursor_entity = Some(new_cursor);
        self.inventory_open = true;
        self.inventory_mode = true;
        self.inv_selected = 20;
    }

    fn exit_inventory(&mut self, ecs: &mut crate::EcsAdapter) {
        self.hide_inventory(ecs);
        self.inventory_open = false;
        self.inventory_mode = false;
        if let Some(old) = self.inv_cursor_entity.take() {
            let _ = ecs.world.entities().delete(old);
            ecs.world.write_storage::<crate::Transform>().remove(old);
            ecs.world.write_storage::<crate::SpriteComponent>().remove(old);
        }
    }

    fn transfer_from_inventory(&mut self, ecs: &mut crate::EcsAdapter) {
        let row = self.inv_selected / 5;
        let col = self.inv_selected % 5;
        let item_idx = ((4 - row) * 5 + col) as usize;
        if item_idx >= INV_ITEMS.len() {
            return;
        }
        let name = INV_ITEMS[item_idx];
        let new_slot = crate::slot_object::make_slot(name);
        let a = self.act_slot as usize;
        if a < self.slots.len() {
            self.slots[a] = new_slot;
            if a < self.slot_entities.len() {
                let path = slot_texture(name);
                ecs.update_sprite_texture(self.slot_entities[a], &path);
            }
        }
        self.exit_inventory(ecs);
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.loaded = false;
        self.loading = true;
        self.loading_text = None;
        self.loading_sprite_key = None;
        self.slots = Vec::new();
        self.act_slot = 0;
        self.mode = 0;
        self.map_size = 1.0;
        self.cursor_entity = None;
        self.icon_mode = None;
        self.icons_slot_cursor = None;
        self.inventory_open = false;
        self.inventory_slots.clear();
        self.inventory_mode = false;
        self.inv_selected = 0;
        self.inv_cursor_entity = None;
        self.slot_entities.clear();
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.loaded {
            if self.loading {
                self.loading = false;
                self.show_loading(ecs, text_renderer, device, queue);
                return SceneAction::None;
            }
            self.hide_loading(ecs);
            self.loaded = true;
            self.setup_content(ecs, text_renderer, device, queue);
        }

        let cursor = self.cursor_entity.unwrap();
        let icon_mode = self.icon_mode.unwrap();
        let icons_slot_cursor = self.icons_slot_cursor.unwrap();

        let result = crate::input::do_input(
            input,
            ecs,
            &mut self.slots,
            self.act_slot,
            self.mode,
            self.map_size,
            window_size,
            cursor,
            icon_mode,
            icons_slot_cursor,
            self.inventory_mode,
        );
        self.act_slot = result.0;
        self.mode = result.1;
        self.map_size = result.2;

        if input.key_pressed(winit::keyboard::KeyCode::KeyE) {
            if self.inventory_open {
                self.exit_inventory(ecs);
            } else {
                self.enter_inventory(ecs);
            }
        }

        if self.inventory_mode {
            if input.key_pressed(winit::keyboard::KeyCode::KeyQ) {
                let col = self.inv_selected % 5;
                let row = self.inv_selected / 5;
                self.inv_selected = if col < 4 {
                    self.inv_selected + 1
                } else if row > 0 {
                    (row - 1) * 5
                } else {
                    20
                };
                let col = self.inv_selected % 5;
                let row = self.inv_selected / 5;
                if let Some(inv_cursor) = self.inv_cursor_entity {
                    ecs.update_transform_position(inv_cursor, -4.0 + col as f32, -3.0 + row as f32);
                }
            }
            if input.key_pressed(winit::keyboard::KeyCode::Enter) {
                self.transfer_from_inventory(ecs);
            }
        }

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }
}
