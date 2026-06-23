use std::collections::HashSet;
use specs::{WorldExt, Builder};
use crate::scene::scene_trait::{Scene, SceneAction};
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use crate::inventory::Inventory;
use crate::pathfinding::{Node, find_path};

fn patrol_routes() -> Vec<Vec<Node>> {
    vec![
        // Route 0: right column down → bottom left → full width right → back up
        vec![
            Node::new(10, -5),  Node::new(10, -11),
            Node::new(3, -11),  Node::new(3, -9),
            Node::new(13, -9),  Node::new(13, -5),
        ],
        // Route 1: bottom full sweep right-to-left, then return
        vec![
            Node::new(13, -9),  Node::new(3, -9),
            Node::new(3, -11),  Node::new(13, -11),
            Node::new(13, -9),
        ],
        // Route 2: right column → bottom right → mid-right → bottom left
        vec![
            Node::new(10, -5),  Node::new(10, -11),
            Node::new(13, -11), Node::new(13, -8),
            Node::new(8, -8),   Node::new(8, -9),
            Node::new(3, -9),   Node::new(3, -11),
        ],
        // Route 3: top-right → bottom sweep → bottom-left → right column up
        vec![
            Node::new(13, -5),  Node::new(13, -9),
            Node::new(3, -9),   Node::new(3, -11),
            Node::new(10, -11), Node::new(10, -5),
        ],
        // Route 4: bottom traverse + right column zigzag
        vec![
            Node::new(3, -9),   Node::new(13, -9),
            Node::new(13, -5),  Node::new(10, -5),
            Node::new(10, -8),  Node::new(13, -8),
            Node::new(13, -11), Node::new(3, -11),
            Node::new(3, -9),
        ],
    ]
}

struct Npc {
    entity: specs::Entity,
    pos: (f32, f32),
    path: Vec<Node>,
    path_index: usize,
    patrol_route: Vec<Node>,
    patrol_index: usize,
    pause: f64,
    walk_timer: f64,
    walk_frame: i32,
}

impl Npc {
    fn new(ecs: &mut crate::EcsAdapter, route: &[Node], start_idx: usize) -> Self {
        let start = route[start_idx];
        let (sx, sy) = start.to_world();
        let entity = ecs.world.create_entity()
            .with(crate::Transform { position: [sx, sy, Z_NPC] })
            .with(crate::SpriteComponent {
                texture_path: TEX_PLAYER_IDLE.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: NPC_SCALE,
            })
            .with(crate::Rotation { rotation: [0.0; 3] })
            .build();
        Npc {
            entity,
            pos: (sx, sy),
            path: Vec::new(),
            path_index: 0,
            patrol_route: route.to_vec(),
            patrol_index: start_idx,
            pause: 0.0,
            walk_timer: 0.0,
            walk_frame: 0,
        }
    }

    fn advance(&mut self, walkable: &HashSet<Node>) {
        loop {
            let (cx, cy) = self.pos;
            let start_node = Node::from_world(cx, cy);
            self.patrol_index = (self.patrol_index + 1) % self.patrol_route.len();
            let goal_node = self.patrol_route[self.patrol_index];
            if start_node == goal_node {
                continue;
            }
            if let Some(path) = find_path(walkable, start_node, goal_node) {
                self.path = path;
                self.path_index = 0;
            }
            break;
        }
    }

    fn set_texture(&self, ecs: &mut crate::EcsAdapter, texture_path: &str) {
        ecs.update_sprite_texture(self.entity, texture_path);
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, dt: f64, walkable: &HashSet<Node>) {
        if self.pause > 0.0 {
            self.pause -= dt;
            self.set_texture(ecs, TEX_PLAYER_IDLE);
            return;
        }

        if self.path_index >= self.path.len() {
            self.set_texture(ecs, TEX_PLAYER_IDLE);
            self.pause = NPC_PAUSE_DURATION;
            self.advance(walkable);
            return;
        }

        self.walk_timer += dt;
        if self.walk_timer > WALK_ANIM_INTERVAL {
            self.walk_timer = 0.0;
            self.walk_frame = 1 - self.walk_frame;
        }
        let tex = if self.walk_frame == 0 { TEX_PLAYER_WALK_1 } else { TEX_PLAYER_WALK_2 };
        self.set_texture(ecs, tex);

        let target = self.path[self.path_index];
        let (tx, ty) = target.to_world();
        let (cx, cy) = self.pos;

        let step = NPC_SPEED * dt as f32;
        let dx = tx - cx;
        let dy = ty - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= step || dist < EPSILON {
            self.pos = (tx, ty);
            self.path_index += 1;
        } else {
            self.pos = (cx + dx / dist * step, cy + dy / dist * step);
        }

        let (nx, ny) = self.pos;
        ecs.update_transform_position(self.entity, nx, ny);

        if dx.abs() > 0.01 {
            let facing = if dx > 0.0 { 0.0 } else { std::f32::consts::PI };
            if let Some(rot) = ecs.world.write_storage::<crate::Rotation>().get_mut(self.entity) {
                rot.rotation = [0.0, facing, 0.0];
            }
        }
    }
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
    npc_walkable: HashSet<Node>,
    npcs: Vec<Npc>,
    last_frame: std::time::Instant,
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

    fn setup_npc(&mut self, ecs: &mut crate::EcsAdapter) {
        self.load_walkable_cells();
        let routes = patrol_routes();
        let start_indices = [0, 0, 7, 0, 0];
        for (idx, route) in routes.iter().enumerate() {
            let start_idx = start_indices[idx.min(start_indices.len() - 1)] % route.len();
            let mut npc = Npc::new(ecs, route, start_idx);
            npc.advance(&self.npc_walkable);
            self.npcs.push(npc);
        }
    }

    fn move_npcs(&mut self, ecs: &mut crate::EcsAdapter, dt: f64) {
        for npc in &mut self.npcs {
            npc.update(ecs, dt, &self.npc_walkable);
        }
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

        let click = input.mouse_pressed(MOUSE_BUTTON_LEFT);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let (wx, wy) = crate::util::ndc_to_world(mx, my, window_size, 1.0);

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
        self.map_size = 0.8;
        self.cursor_entity = None;
        self.icon_mode = None;
        self.icons_slot_cursor = None;
        self.slot_entities.clear();
        self.inventory.reset();
        self.npc_walkable.clear();
        self.npcs.clear();
        self.last_frame = std::time::Instant::now();
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

        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;
        self.move_npcs(ecs, dt);

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }
}
