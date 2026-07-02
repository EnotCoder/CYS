use std::collections::HashSet;
use specs::{WorldExt, Join};
use winit::keyboard::KeyCode;
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use crate::inventory::Inventory;
use crate::pathfinding::Node;
use crate::ecs::components::{FoodStorage, ObjectTag, TotalFood, BusyCassas, Transform, Money};
use crate::shopper_npc::ShopperNpc;

pub struct GameScene {
    loaded: bool,
    loading: bool,
    loading_text: Option<specs::Entity>,
    loading_sprite_key: Option<u64>,
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
    last_frame: std::time::Instant,
    anim_timer: f64,
    camera_offset_x: f32,
    camera_offset_y: f32,
    ilm_entity: Option<specs::Entity>,
    ilm_timer: f64,
    ilm_cooldown: f64,
    food_timer: f64,
    total_food_text: Option<specs::Entity>,
    total_food_sprite_key: Option<u64>,
    current_total_food: i32,
    money_text: Option<specs::Entity>,
    money_sprite_key: Option<u64>,
    current_money: i32,
    object_hover_text: Option<specs::Entity>,
    slot_tooltip_text: Option<specs::Entity>,
    slot_tooltip_text_key: Option<u64>,
    slot_tooltip_bg: Option<specs::Entity>,
    slot_tooltip_bg_key: Option<u64>,
    shoppers: Vec<ShopperNpc>,
    shopper_timer: f64,
    active: bool,
    active_entity: Option<specs::Entity>,
    settings: crate::settings::Settings,
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
            money_text: None,
            money_sprite_key: None,
            current_money: -1,
            object_hover_text: None,
            slot_tooltip_text: None,
            slot_tooltip_text_key: None,
            slot_tooltip_bg: None,
            slot_tooltip_bg_key: None,
            shoppers: Vec::new(),
            shopper_timer: 0.0,
            active: true,
            active_entity: None,
            settings: crate::settings::Settings::new(),
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

        text_renderer.add_text(ecs, device, queue, "Pre alpha", FONT_SIZE_LOGO, -5.0, 4.0, 2.0, 4.0, WHITE);

        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[0]);
        let active_entity = ecs.add_ui(ACTIVE_X, SLOT_BAR_Y, TEX_ACTIVE);
        self.active_entity = Some(active_entity);

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

        let click = input.mouse_pressed(winit::event::MouseButton::Left);
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
            if self.inventory.handle_grid_click(col, row) {
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
        self.money_text = None;
        self.money_sprite_key = None;
        self.current_money = -1;
        self.object_hover_text = None;
        self.slot_tooltip_text = None;
        self.slot_tooltip_text_key = None;
        self.slot_tooltip_bg = None;
        self.slot_tooltip_bg_key = None;
        self.shoppers.clear();
        self.shopper_timer = 0.0;
        self.active = true;
        self.active_entity = None;
        self.settings = crate::settings::Settings::new();
        ecs.world.write_resource::<TotalFood>().0 = 0;
        ecs.world.write_resource::<BusyCassas>().0.clear();
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

        // --- Toggle settings ---
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            if self.settings.open {
                self.settings.close(ecs);
            } else {
                self.settings.open(ecs, text_renderer, device, queue);
            }
        }

        if self.settings.open {
            self.settings.handle_input(ecs, text_renderer, device, queue, input, window_size);
            if self.settings.vsync_toggled {
                self.settings.vsync_toggled = false;
                let enabled = self.settings.vsync.checked;
                return SceneAction::VsyncToggle(enabled);
            }
        } else {
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

            // --- Toggle active/not active ---
            if input.mouse_pressed(winit::event::MouseButton::Left) && !self.inventory.mode {
                if let Some((mx, my)) = input.cursor() {
                    let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
                    if (wx - ACTIVE_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        self.active = !self.active;
                        let tex = if self.active { TEX_ACTIVE } else { TEX_NO_ACTIVE };
                        if let Some(entity) = self.active_entity {
                            ecs.update_sprite_texture(entity, tex);
                        }
                        if !self.active {
                            for shopper in &mut self.shoppers {
                                shopper.set_exiting(true);
                            }
                        } else {
                            for shopper in &mut self.shoppers {
                                if !shopper.has_taken_food() {
                                    shopper.set_exiting(false);
                                }
                            }
                        }
                    }
                    // --- Клик по mode ---
                    if (wx - ICON_MODE_X).abs() < TILE_HALF && (wy - SLOT_BAR_Y).abs() < TILE_HALF {
                        let cursor = self.cursor_entity.unwrap();
                        let icon_mode = self.icon_mode.unwrap();
                        self.mode = crate::input::interact::cycle_mode(self.mode, ecs, cursor, icon_mode);
                    }
                }
            }

            if show_ilm && self.ilm_cooldown <= 0.0 && self.ilm_entity.is_none() {
                let ent = text_renderer.add_text(ecs, device, queue, "Minecraft", 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
                self.ilm_entity = Some(ent);
                self.ilm_timer = 2.0;
                self.ilm_cooldown = 5.0;
            }

            self.handle_inventory_input(ecs, input, window_size);
        }

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

        if input.mouse_held(winit::event::MouseButton::Middle) {
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

        // --- Обновление всех объектов по компонентам ---
        self.food_timer += dt;
        if self.food_timer >= 1.0 {
            self.food_timer -= 1.0;
            {
                let tags = ecs.world.read_storage::<ObjectTag>();
                let mut foods = ecs.world.write_storage::<FoodStorage>();
                for (tag, storage) in (&tags, &mut foods).join() {
                    if tag.name == "box" && storage.food_count < storage.max_food {
                        storage.food_count += 1;
                    }
                }
            }
            ecs.update_object_textures();
        }
        ecs.update_fence_textures();
        let cursor_pos = self.cursor_entity.map(|e| ecs.get_transform_position(e));
        let hovered_object = cursor_pos.and_then(|(cx, cy)| {
            let gx = cx as i32;
            let gy = cy as i32;
            if let Some(gid) = ecs.find_group_at_position(gx, gy) {
                let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
                if let Some(info) = groups.groups.get(&gid) {
                    if let Some(first) = info.entities.first() {
                        let foods = ecs.world.read_storage::<FoodStorage>();
                        let tags = ecs.world.read_storage::<ObjectTag>();
                        if let Some(f) = foods.get(*first) {
                            let name = tags.get(*first).map(|t| t.name.clone()).unwrap_or("Object".to_string());
                            return Some((f.food_count, f.max_food, name));
                        }
                    }
                }
            }
            None
        });
        if let Some((food, max, name)) = hovered_object {
            let text = format!("{}: {}/{}", name, food, max);
            if let Some(old_ent) = self.object_hover_text.take() {
                ecs.delete_entity(old_ent);
            }
            let ent = text_renderer.add_text(ecs, device, queue, &text, 48.0, 0.0, -3.0, 2.0, 1.0, WHITE);
            self.object_hover_text = Some(ent);
        } else if let Some(ent) = self.object_hover_text.take() {
            ecs.delete_entity(ent);
        }
        // --- Tooltip для ячеек инвентаря ---
        let slot_tooltip = if self.inventory.mode {
            input.cursor().and_then(|(mx, my)| {
                let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);
                let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
                let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
                if col >= 0 && col < INVENTORY_COLS && row >= 0 && row < INVENTORY_ROWS {
                    let item_idx = crate::util::inventory_index(row, col) as usize;
                    let items = self.inventory.items();
                    if item_idx < items.len() {
                        let name = items[item_idx];
                        Some((name, SLOT_BAR_X + col as f32, INVENTORY_BASE_Y + row as f32 - 0.55))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            None
        };
        if let Some((name, tx, ty)) = slot_tooltip {
            if let Some(old) = self.slot_tooltip_text.take() {
                ecs.delete_entity(old);
            }
            if let Some(old_key) = self.slot_tooltip_text_key.take() {
                ecs.sprite_cache.remove(&old_key);
            }
            if let Some(old_bg) = self.slot_tooltip_bg.take() {
                ecs.delete_entity(old_bg);
            }
            if let Some(old_key) = self.slot_tooltip_bg_key.take() {
                ecs.sprite_cache.remove(&old_key);
            }
            let display_name = name.replace('_', " ");
            let char_w = 0.14;
            let text_w = (display_name.len() as f32).max(1.0) * char_w;
            let (_, text_h) = text_renderer.text_world_size(&display_name, FONT_SIZE_LOGO, text_w, 4.0, WHITE);
            let pad_x = 0.10;
            let pad_y = 0.04;
            let bg_w = text_w + pad_x * 2.0;
            let bg_h = text_h + pad_y * 2.0;
            let bg_ent = ecs.add_ui_sized(tx, ty, bg_w, bg_h, "tex/black.png", device, queue);
            self.slot_tooltip_bg = Some(bg_ent);
            let text_ent = text_renderer.add_text_fixed(ecs, device, queue, &display_name, FONT_SIZE_LOGO, tx, ty, text_w, text_h, 4.0, WHITE);
            self.slot_tooltip_text = Some(text_ent);
            let text_key = TextRenderer::sprite_cache_key(tx, ty, &display_name, FONT_SIZE_LOGO, 4.0, WHITE);
            self.slot_tooltip_text_key = Some(text_key);
            let bg_key = crate::util::sprite_cache_key("ui", tx, ty, "tex/black.png", [0, 0], [1, 1], 1.0);
            self.slot_tooltip_bg_key = Some(bg_key);
        } else {
            if let Some(old) = self.slot_tooltip_text.take() {
                ecs.delete_entity(old);
            }
            if let Some(old_key) = self.slot_tooltip_text_key.take() {
                ecs.sprite_cache.remove(&old_key);
            }
            if let Some(old_bg) = self.slot_tooltip_bg.take() {
                ecs.delete_entity(old_bg);
            }
            if let Some(old_key) = self.slot_tooltip_bg_key.take() {
                ecs.sprite_cache.remove(&old_key);
            }
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
                let ent = text_renderer.add_text(ecs, device, queue, &text, 64.0, 5.75, 3.5, 1.0, 4.0, WHITE);
                self.total_food_text = Some(ent);
                self.total_food_sprite_key = Some(TextRenderer::sprite_cache_key(5.75, 3.5, &text, 24.0, 1.0, GREEN));
            }
        }
        {
            let money = ecs.world.read_resource::<Money>().0;
            if money != self.current_money {
                self.current_money = money;
                if let Some(entity) = self.money_text.take() {
                    ecs.delete_entity(entity);
                }
                if let Some(key) = self.money_sprite_key.take() {
                    ecs.sprite_cache.remove(&key);
                }
                let text = format!("Money: {}", money);
                let ent = text_renderer.add_text(ecs, device, queue, &text, 64.0, 5.75, 3.0, 1.0, 4.0, WHITE);
                self.money_text = Some(ent);
                self.money_sprite_key = Some(TextRenderer::sprite_cache_key(5.75, 3.0, &text, 24.0, 1.0, GREEN));
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

        // --- Shopper NPCs ---
        self.shopper_timer += dt;
        if self.active && self.shopper_timer >= 3.0 && self.shoppers.len() < MAX_SHOPPERS {
            self.shopper_timer = 0.0;
            let (all_racks, all_cassas) = {
                let tags = ecs.world.read_storage::<ObjectTag>();
                let foods = ecs.world.read_storage::<FoodStorage>();
                let transforms = ecs.world.read_storage::<Transform>();
                let mut racks = Vec::new();
                let mut cassas = Vec::new();
                for (tag, transform) in (&tags, &transforms).join() {
                    if tag.name == "cassa" {
                        cassas.push(Node::new(transform.position[0] as i32, transform.position[1] as i32));
                    }
                }
                for (tag, food, transform) in (&tags, &foods, &transforms).join() {
                    if tag.name == "rack" && food.food_count > 0 {
                        racks.push(Node::new(transform.position[0] as i32, transform.position[1] as i32 + 1));
                    }
                }
                (racks, cassas)
            };
            if !all_racks.is_empty() && !all_cassas.is_empty() {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
                let rack = all_racks[(seed as usize) % all_racks.len()];
                let cassa = all_cassas[((seed >> 16) as usize) % all_cassas.len()];
                let spawn_node = crate::map_loader::shopper_spawn_point();
                if let Some(shopper) = ShopperNpc::spawn(ecs, &self.npc_walkable, spawn_node, rack, cassa) {
                    self.shoppers.push(shopper);
                }
            }
        }
        self.shoppers.retain_mut(|shopper| {
            let done = shopper.update(ecs, dt, &self.npc_walkable);
            if done {
                shopper.despawn(ecs);
            }
            !done
        });

        self.update_animations(ecs, dt);

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter, visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer(visible_bounds)
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }

    fn camera_offset(&self) -> (f32, f32) {
        (self.camera_offset_x, self.camera_offset_y)
    }
}
