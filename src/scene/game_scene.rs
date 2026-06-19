use crate::scene::{Scene, SceneAction};

pub struct GameScene {
    loaded: bool,
    slots: Vec<crate::slot_object::Slot>,
    act_slot: i32,
    mode: i32,
    map_size: f32,
    cursor_entity: Option<specs::Entity>,
    icon_mode: Option<specs::Entity>,
    icons_slot_cursor: Option<specs::Entity>,
}

impl GameScene {
    pub fn new() -> Self {
        GameScene {
            loaded: false,
            slots: Vec::new(),
            act_slot: 0,
            mode: 0,
            map_size: 0.8,
            cursor_entity: None,
            icon_mode: None,
            icons_slot_cursor: None,
        }
    }

    fn init_game(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        crate::load_map_to_ecs(ecs);

        self.slots = crate::slot_object::get_slot_vec();

        let (icon_mode, icons_slot_cursor) = get_uv_ecs(ecs, &self.slots, text_renderer, device, queue);
        self.icon_mode = Some(icon_mode);
        self.icons_slot_cursor = Some(icons_slot_cursor);
        self.cursor_entity = Some(ecs.add_cursor(0.0, 0.0, "tex/cursor/def_cursor.png"));
    }
}

fn get_uv_ecs(ecs: &mut crate::EcsAdapter, slots: &[crate::slot_object::Slot], text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> (specs::Entity, specs::Entity) {
    text_renderer.add_text(
        ecs, device, queue,
        "Pre alpha", 128.0, -3.0, 3.8, 2.0, 4.0, [255, 255, 255],
    );

    let icon_mode = ecs.add_ui(4.0, -4.0, "tex/ui/mode/standart_mode.png");

    for (i, slot) in slots.iter().enumerate() {
        ecs.add_ui(
            -4.0 + i as f32,
            -4.0,
            &format!("tex/ui/icon_slots/{}.png", slot.obj.name),
        );
    }

    let icons_slot_cursor = ecs.add_ui(-4.0, -4.0, "tex/ui/icon_slots/cursor.png");

    (icon_mode, icons_slot_cursor)
}

impl Scene for GameScene {
    fn on_enter(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::text_renderer::TextRenderer) {
        self.loaded = false;
        self.slots = Vec::new();
        self.act_slot = 0;
        self.mode = 0;
        self.map_size = 0.8;
        self.cursor_entity = None;
        self.icon_mode = None;
        self.icons_slot_cursor = None;
    }

    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction {
        if !self.loaded {
            self.loaded = true;
            self.init_game(ecs, text_renderer, device, queue);
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
        );
        self.act_slot = result.0;
        self.mode = result.1;
        self.map_size = result.2;

        SceneAction::None
    }

    fn sprites(&self, ecs: &crate::EcsAdapter) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>) {
        ecs.get_sprites_by_layer()
    }

    fn map_size(&self) -> f32 {
        self.map_size
    }
}
