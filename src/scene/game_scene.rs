use std::collections::HashSet;
use specs::{WorldExt, Join};
use winit::keyboard::KeyCode;
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use crate::inventory::Inventory;
use crate::pathfinding::Node;
use crate::npc::Npc;
use crate::ecs::components::{BoxStorage, TotalFood};

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
    slot_entities: Vec<specs::Entity>,
    inventory: Inventory,
    npc_walkable: HashSet<Node>,
    npcs: Vec<Npc>,
    last_frame: std::time::Instant,
    anim_timer: f64,
    camera_offset_x: f32,
    camera_offset_y: f32,
    ilm_entity: Option<specs::Entity>,
    ilm_timer: f64,
    ilm_cooldown: f64,
    food_timer: f64,
    total_food_text: Option<specs::Entity>,
    total_food_sprite_key: Option<String>,
    current_total_food: i32,
    box_hover_text: Option<specs::Entity>,
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
            map_size: 0.8,
            cursor_entity: None,
            icon_mode: None,
            icons_slot_cursor: None,
            slot_entities: Vec::new(),
            inventory: Inventory::new(),
            npc_walkable: HashSet::new(),
            npcs: Vec::new(),
            last_frame: std::time::Instant::now(),
            anim_timer: 0.0,
            camera_offset_x: 0.0,
            camera_offset_y: 0.0,
            ilm_entity: None,
            ilm_timer: 0.0,
            ilm_cooldown: 0.0,
            food_timer: 0.0,
            total_food_text: None,
            total_food_sprite_key: None,
            current_total_food: -1,
            box_hover_text: None,
        }
    }

    fn show_loading(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        let entity = text_renderer.add_text(ecs, device, queue, "Loading...", 64.0, 0.0, 0.0, 4.0, 2.0, GRAY);
        self.loading_text = Some(entity);
        self.loading_sprite_key = Some(TextRenderer::sprite_cache_key(0.0, 0.0, "Loading...", 48.0, 2.0, GRAY));
    }

    fn hide_loading(&mut self, ecs: &mut crate::EcsAdapter) {
        if let Some(entity) = self.loading_text.take() {
            ecs.delete_entity(entity);
        }
        if let Some(key) = self.loading_sprite_key.take() {
            ecs.sprite_cache.remove(&key);
        }
    }

    fn setup_content(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);

        self.slots = crate::slot_object::get_slot_vec();

        text_renderer.add_text(ecs, device, queue, "Pre alpha", FONT_SIZE_LOGO, -4.0, 4.0, 2.0, 4.0, WHITE);

        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[0]);

        for (i, slot) in self.slots.iter().enumerate() {
            let ent = ecs.add_ui(
                SLOT_BAR_X + i as f32, SLOT_BAR_Y,
                &format!("{}{}.png", TEX_UI_ICON_SLOTS_DIR, slot.obj.name),
            );
            self.slot_entities.push(ent);
        }

        let icons_slot_cursor = ecs.add_ui(SLOT_BAR_X, SLOT_BAR_Y, SLOT_CURSOR_TEX);
        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, CURSOR_TEX[0]));

        self.npc_walkable = crate::map_loader::load_walkable_cells();
        self.npcs = crate::npc::setup_npcs(ecs, &self.npc_walkable);
    }

    fn update_animations(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        self.anim_timer += dt;
        if self.anim_timer >= 1.0 {
            self.anim_timer -= 1.0;
            let mut sprites = ecs.world.write_storage::<crate::SpriteComponent>();
            for sprite in (&mut sprites).join() {
                if sprite.animated {
                    let n = sprite.frame_paths.len() as i32;
                    sprite.current_frame = (sprite.current_frame + 1) % n;
                    if let Some(path) = sprite.frame_paths.get(sprite.current_frame as usize) {
                        sprite.texture_path = path.clone();
                    }
                }
            }
        }
    }

    fn handle_inventory_input(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) {
        if input.key_pressed(winit::keyboard::KeyCode::KeyE) {
            if self.inventory.open {
                self.inventory.exit(ecs);
            } else {
                self.inventory.enter(ecs);
            }
        }

        let click = input.mouse_pressed(MOUSE_BUTTON_LEFT);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);

        // --- Клик по табам ---
        if self.inventory.mode {
            let tcol = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            if (wy - INV_TAB_Y).abs() < TILE_HALF && tcol >= 0 && tcol < TAB_TEX.len() as i32 {
                if tcol != self.inventory.tab {
                    self.inventory.switch_tab(tcol, ecs);
                }
                return;
            }
        }

        // --- Клик по сетке инвентаря ---
        if self.inventory.mode {
            let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
            if self.inventory.handle_grid_click(col, row, ecs) {
                self.inventory.transfer_to_slot(ecs, self.act_slot as usize, &mut self.slots, &self.slot_entities);
            }
            return;
        }

        // --- Клик по слотам хотбара ---
        let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
        if (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < self.slots.len() as i32 {
            let target = col;
            if target != self.act_slot {
                if let Some(cursor) = self.icons_slot_cursor {
                    let old = self.act_slot as usize;
                    if old < self.slots.len() {
                        self.slots[old].active = false;
                    }
                    self.act_slot = target;
                    self.slots[target as usize].active = true;
                    ecs.update_transform_position(cursor, SLOT_BAR_X + col as f32, SLOT_BAR_Y);
                }
            }
        }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.loaded = false;
        self.loading = true;
        self.loading_text = None;
        self.loading_sprite_key = None;
        self.slots = Vec::new();
        self.act_slot = 0;
        self.mode = 0;
        self.map_size = 0.8;
        self.cursor_entity = None;
        self.icon_mode = None;
        self.icons_slot_cursor = None;
        self.slot_entities.clear();
        self.inventory.reset();
        self.npc_walkable.clear();
        self.npcs.clear();
        self.last_frame = std::time::Instant::now();
        self.anim_timer = 0.0;
        self.camera_offset_x = 0.0;
        self.camera_offset_y = 0.0;
        self.ilm_entity = None;
        self.ilm_timer = 0.0;
        self.ilm_cooldown = 0.0;
        self.food_timer = 0.0;
        self.total_food_text = None;
        self.total_food_sprite_key = None;
        self.current_total_food = -1;
        self.box_hover_text = None;
        ecs.world.write_resource::<BoxStorage>().boxes.clear();
        ecs.world.write_resource::<TotalFood>().0 = 0;
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
            input, ecs, &mut self.slots, self.act_slot, self.mode, self.map_size,
            window_size, cursor, icon_mode, icons_slot_cursor, self.inventory.mode,
            self.camera_offset_x, self.camera_offset_y,
        );
        self.act_slot = result.0;
        self.mode = result.1;
        self.map_size = result.2;
        let show_ilm = result.3;

        if show_ilm && self.ilm_cooldown <= 0.0 && self.ilm_entity.is_none() {
            let ent = text_renderer.add_text(ecs, device, queue, "Minecraft", 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
            self.ilm_entity = Some(ent);
            self.ilm_timer = 2.0;
            self.ilm_cooldown = 5.0;
        }

        self.handle_inventory_input(ecs, input, window_size);

        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;

        let aspect = window_size.0 / window_size.1;
        let vis_w = 2.0 * aspect / (SHADER_SCALE * self.map_size);
        let vis_h = 2.0 / (SHADER_SCALE * self.map_size);
        let cam_min_x = CAMERA_MAP_MIN_X + vis_w / 2.0;
        let cam_max_x = CAMERA_MAP_MAX_X - vis_w / 2.0;
        let cam_min_y = CAMERA_MAP_MIN_Y + vis_h / 2.0;
        let cam_max_y = CAMERA_MAP_MAX_Y - vis_h / 2.0;

        let step = CAMERA_SPEED * (dt as f32);

        if input.mouse_held(MOUSE_BUTTON_MIDDLE) {
            let sensitivity = 0.01;
            let (dx, dy) = input.cursor_diff();
            self.camera_offset_x = (self.camera_offset_x - dx * sensitivity).clamp(cam_min_x.min(cam_max_x), cam_min_x.max(cam_max_x));
            self.camera_offset_y = (self.camera_offset_y + dy * sensitivity).clamp(cam_min_y.min(cam_max_y), cam_min_y.max(cam_max_y));
        }

        if input.key_held(KeyCode::ArrowLeft) {
            self.camera_offset_x = (self.camera_offset_x - step).max(cam_min_x);
        }
        if input.key_held(KeyCode::ArrowRight) {
            self.camera_offset_x = (self.camera_offset_x + step).min(cam_max_x);
        }
        if input.key_held(KeyCode::ArrowDown) {
            self.camera_offset_y = (self.camera_offset_y - step).max(cam_min_y);
        }
        if input.key_held(KeyCode::ArrowUp) {
            self.camera_offset_y = (self.camera_offset_y + step).min(cam_max_y);
        }
        self.camera_offset_x = self.camera_offset_x.clamp(cam_min_x.min(cam_max_x), cam_min_x.max(cam_max_x));
        self.camera_offset_y = self.camera_offset_y.clamp(cam_min_y.min(cam_max_y), cam_min_y.max(cam_max_y));

        // --- Box food system ---
        self.food_timer += dt;
        if self.food_timer >= 1.0 {
            self.food_timer -= 1.0;
            let mut storage = ecs.world.write_resource::<BoxStorage>();
            for (_, data) in storage.boxes.iter_mut() {
                if data.food_count < data.max_food {
                    data.food_count += 1;
                }
            }
        }
        let cursor_pos = self.cursor_entity.map(|e| ecs.get_transform_position(e));
        let hovered_box = cursor_pos.and_then(|(cx, cy)| {
            let gx = cx as i32;
            let gy = cy as i32;
            let storage = ecs.world.read_resource::<BoxStorage>();
            storage.boxes.iter().find(|(_, d)| d.pos_x == gx && d.pos_y == gy).map(|(&gid, d)| (gid, d.food_count, d.max_food))
        });
        if let Some((_, food, max)) = hovered_box {
            let text = format!("Box: {}/{}", food, max);
            if let Some(old_ent) = self.box_hover_text.take() {
                ecs.delete_entity(old_ent);
            }
            let ent = text_renderer.add_text(ecs, device, queue, &text, 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
            self.box_hover_text = Some(ent);
        } else if let Some(ent) = self.box_hover_text.take() {
            ecs.delete_entity(ent);
        }
        {
            let total = ecs.world.read_resource::<TotalFood>().0;
            if total != self.current_total_food {
                self.current_total_food = total;
                if let Some(entity) = self.total_food_text.take() {
                    ecs.delete_entity(entity);
                }
                if let Some(key) = self.total_food_sprite_key.take() {
                    ecs.sprite_cache.remove(&key);
                }
                let text = format!("Food: {}", total);
                let ent = text_renderer.add_text(ecs, device, queue, &text, 64.0, 5.0, 3.5, 1.0, 4.0, WHITE);
                self.total_food_text = Some(ent);
                self.total_food_sprite_key = Some(TextRenderer::sprite_cache_key(5.0, 3.5, &text, 24.0, 1.0, GREEN));
            }
        }

        if self.ilm_cooldown > 0.0 {
            self.ilm_cooldown -= dt;
        }
        if let Some(ent) = self.ilm_entity {
            self.ilm_timer -= dt;
            if self.ilm_timer <= 0.0 {
                ecs.delete_entity(ent);
                self.ilm_entity = None;
            }
        }

        crate::npc::move_npcs(&mut self.npcs, ecs, dt, &self.npc_walkable);
        self.update_animations(ecs, dt);

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }

    fn camera_offset(&self) -> (f32, f32) {
        (self.camera_offset_x, self.camera_offset_y)
    }
}
