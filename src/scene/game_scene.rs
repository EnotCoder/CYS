use std::collections::HashSet;
use specs::{WorldExt, Builder};
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use crate::inventory::Inventory;
use crate::pathfinding::{Node, find_path};

fn patrol_route() -> Vec<Node> {
    vec![
        Node::new(10, -5),
        Node::new(10, -11),
        Node::new(3, -11),
        Node::new(3, -8),
    ]
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
    slot_entities: Vec<specs::Entity>,
    inventory: Inventory,
    npc_entity: Option<specs::Entity>,
    npc_walkable: HashSet<Node>,
    npc_pos: (f32, f32),
    npc_path: Vec<Node>,
    npc_path_index: usize,
    npc_patrol_index: usize,
    npc_pause: f64,
    last_frame: std::time::Instant,
    walk_anim_timer: f64,
    walk_frame: i32,
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
            slot_entities: Vec::new(),
            inventory: Inventory::new(),
            npc_entity: None,
            npc_walkable: HashSet::new(),
            npc_pos: (0.0, 0.0),
            npc_path: Vec::new(),
            npc_path_index: 0,
            npc_patrol_index: 0,
            npc_pause: 0.0,
            last_frame: std::time::Instant::now(),
            walk_anim_timer: 0.0,
            walk_frame: 0,
        }
    }

    // ====================================================================
    //  Загрузка / выгрузка
    // ====================================================================

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

        text_renderer.add_text(ecs, device, queue, "Pre alpha", 128.0, -4.0, 4.0, 2.0, 4.0, WHITE);

        let icon_mode = ecs.add_ui(ICON_MODE_X, SLOT_BAR_Y, MODE_ICON_TEX[0]);

        for (i, slot) in self.slots.iter().enumerate() {
            let ent = ecs.add_ui(
                SLOT_BAR_X + i as f32, SLOT_BAR_Y,
                &format!("tex/ui/icon_slots/{}.png", slot.obj.name),
            );
            self.slot_entities.push(ent);
        }

        let icons_slot_cursor = ecs.add_ui(SLOT_BAR_X, SLOT_BAR_Y, "tex/ui/icon_slots/cursor.png");
        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, CURSOR_TEX[0]));

        self.setup_npc(ecs);
    }

    // ====================================================================
    //  NPC / patrol with A*
    // ====================================================================

    fn load_walkable_cells(&mut self) {
        let src = include_str!("../../map.txt");
        for (j, line) in src.lines().enumerate() {
            for (i, token) in line.split_whitespace().enumerate() {
                if token == "@" && i >= 18 && j >= 16 {
                    let wx = i as f32 + WORLD_OFFSET_X;
                    let wy = -(j as f32) + WORLD_OFFSET_Y;
                    self.npc_walkable.insert(Node::from_world(wx, wy));
                }
            }
        }
    }

    fn advance_patrol(&mut self) {
        loop {
            let (cx, cy) = self.npc_pos;
            let start_node = Node::from_world(cx, cy);
            let route = patrol_route();
            self.npc_patrol_index = (self.npc_patrol_index + 1) % route.len();
            let goal_node = route[self.npc_patrol_index];
            if start_node == goal_node {
                continue;
            }
            if let Some(path) = find_path(&self.npc_walkable, start_node, goal_node) {
                self.npc_path = path;
                self.npc_path_index = 0;
            }
            break;
        }
    }

    fn setup_npc(&mut self, ecs: &mut crate::EcsAdapter) {
        self.load_walkable_cells();
        let start = patrol_route()[0];
        let (sx, sy) = start.to_world();
        self.npc_pos = (sx, sy);
        self.npc_patrol_index = 0;
        let entity = ecs.world.create_entity()
            .with(crate::Transform { position: [sx, sy, Z_NPC] })
            .with(crate::SpriteComponent {
                texture_path: "tex/characters/player.png".to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: 1.5,
            })
            .build();
        self.npc_entity = Some(entity);
        self.advance_patrol();
    }

    fn set_npc_texture(&self, ecs: &mut crate::EcsAdapter, texture_path: &str) {
        if let Some(entity) = self.npc_entity {
            ecs.update_sprite_texture(entity, texture_path);
        }
    }

    fn move_npc(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        let Some(entity) = self.npc_entity else { return };

        if self.npc_pause > 0.0 {
            self.npc_pause -= dt;
            self.set_npc_texture(ecs, "tex/characters/player.png");
            return;
        }

        if self.npc_path_index >= self.npc_path.len() {
            self.set_npc_texture(ecs, "tex/characters/player.png");
            self.npc_pause = 0.3;
            self.advance_patrol();
            return;
        }

        // Ходьба — анимируем текстуру
        self.walk_anim_timer += dt;
        if self.walk_anim_timer > 0.3 {
            self.walk_anim_timer = 0.0;
            self.walk_frame = 1 - self.walk_frame;
        }
        let tex = if self.walk_frame == 0 {
            "tex/characters/player_walk_1.png"
        } else {
            "tex/characters/player_walk_2.png"
        };
        self.set_npc_texture(ecs, tex);

        let target = self.npc_path[self.npc_path_index];
        let (tx, ty) = target.to_world();
        let (cx, cy) = self.npc_pos;

        let speed = 3.0;
        let step = speed * dt as f32;
        let dx = tx - cx;
        let dy = ty - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= step || dist < 0.01 {
            self.npc_pos = (tx, ty);
            self.npc_path_index += 1;
        } else {
            self.npc_pos = (cx + dx / dist * step, cy + dy / dist * step);
        }

        let (nx, ny) = self.npc_pos;
        ecs.update_transform_position(entity, nx, ny);
    }

    // ====================================================================
    //  Инвентарь + хотбар — ввод
    // ====================================================================

    fn handle_inventory_input(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32)) {
        if input.key_pressed(winit::keyboard::KeyCode::KeyE) {
            if self.inventory.open {
                self.inventory.exit(ecs);
            } else {
                self.inventory.enter(ecs);
            }
        }

        let click = input.mouse_pressed(0);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let scale = crate::constants::SHADER_SCALE;
        let aspect = window_size.0 / window_size.1;
        let wx = ((mx / window_size.0) * 2.0 - 1.0) * aspect / scale;
        let wy = (1.0 - (my / window_size.1) * 2.0) / scale;

        // --- Клик по табам ---
        if self.inventory.mode {
            let tcol = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            if (wy - INV_TAB_Y).abs() < TILE_HALF && (tcol == 0 || tcol == 1) {
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
        self.slot_entities.clear();
        self.inventory.reset();
        self.npc_entity = None;
        self.npc_walkable.clear();
        self.npc_path.clear();
        self.npc_patrol_index = 0;
        self.npc_pause = 0.0;
        self.last_frame = std::time::Instant::now();
        self.walk_anim_timer = 0.0;
        self.walk_frame = 0;
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
        );
        self.act_slot = result.0;
        self.mode = result.1;
        self.map_size = result.2;

        self.handle_inventory_input(ecs, input, window_size);

        if self.npc_entity.is_some() {
            let now = std::time::Instant::now();
            let dt = (now - self.last_frame).as_secs_f64();
            self.last_frame = now;
            self.move_npc(ecs, dt);
        }

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }
}
