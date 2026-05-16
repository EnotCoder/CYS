// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_panel: true,
        }
    }
    
    pub fn render(&mut self, ctx: &Context) {
        if !self.show_panel {
            return;
        }
        
        // Главное окно управления
        Window::new("Control Panel")
            .default_pos([10.0, 10.0])
            .default_size([280.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Model Controls");
            });
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }
}