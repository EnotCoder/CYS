// ui_panels.rs
use egui::{Context, Window};

pub struct UiState {
    pub show_panel: bool,
}

impl UiState {
    pub fn new(
    ) -> Self {
        Self {
            show_panel: true,
        }
    }
    
    pub fn render(&mut self, ctx: &Context) {
        if !self.show_panel {
            return;
        }
        
        // Главное окно управления
        Window::new("TMV Alpha")
            .default_pos([10.0, 10.0])
            .default_size([280.0, 400.0])
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .show(ctx, |ui| {
                ui.heading("Create your shop (V-Test)");
            });
    }
}