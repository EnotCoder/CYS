use specs::Entity;
use crate::ui::{Panel, Checkbox, create_panel, destroy_panel, create_checkbox, destroy_checkbox, refresh_checkbox, checkbox_clicked};
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use crate::EcsAdapter;
use winit_input_helper::WinitInputHelper;

pub struct Settings {
    pub open: bool,
    pub panel: Panel,
    pub title: Option<Entity>,
    pub vsync: Checkbox,
    /// Флаг, что vsync изменился — сцена вернёт SceneAction
    pub vsync_toggled: bool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: Panel::new(0.0, 0.0, 9.0, 6.0),
            title: None,
            vsync: Checkbox::new(-1.4, 0.3, "Vertical Sync", true),
            vsync_toggled: false,
        }
    }

    pub fn open(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.open { return; }
        self.open = true;
        create_panel(ecs, device, queue, &mut self.panel);
        create_checkbox(ecs, text_renderer, device, queue, &mut self.vsync);
        let title = text_renderer.add_text(ecs, device, queue, "Settings", 64.0, 0.0, 1.8, 4.0, 2.0, WHITE);
        self.title = Some(title);
    }

    pub fn close(&mut self, ecs: &mut EcsAdapter) {
        if !self.open { return; }
        self.open = false;
        destroy_panel(ecs, &mut self.panel);
        destroy_checkbox(ecs, &mut self.vsync);
        if let Some(ent) = self.title.take() {
            ecs.delete_entity(ent);
        }
    }

    /// Обработка кликов по настройкам.
    /// Возвращает true, если клик был обработан (настройки перехватили ввод).
    pub fn handle_input(&mut self, ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, input: &WinitInputHelper, window_size: (f32, f32)) -> bool {
        if !self.open { return false; }

        if checkbox_clicked(&self.vsync, input, window_size) {
            self.vsync.checked = !self.vsync.checked;
            refresh_checkbox(ecs, text_renderer, device, queue, &mut self.vsync);
            self.vsync_toggled = true;
            return true;
        }

        false
    }
}
