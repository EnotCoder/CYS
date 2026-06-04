// ui_panels.rs
use egui::{Context, Window};

use crate::Slot;

pub struct UiState {
    pub show_panel: bool,
    pub slots: Vec<Slot>,
}

impl UiState {
    pub fn new(
        slots: Vec<Slot>,
    ) -> Self {
        Self {
            show_panel: true,
            slots,
        }
    }
    
    pub fn render(&mut self, ctx: &Context) {
        if !self.show_panel {
            return;
        }
        
        // Главное окно управления
        Window::new("CYS Pre-Alpha")
            .default_pos([10.0, 10.0])
            .default_size([280.0, 400.0])
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .show(ctx, |ui| {
                ui.heading("Create your shop (V-Pre Alpha)");
                ui.separator();
                for slot in &self.slots{
                    ui.label(format!("{}: {}, active is {}", slot.id, slot.obj.name, slot.active));
                }   

            });
    }
}