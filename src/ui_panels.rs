// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
    pub model_path: String,
    pub texture_path: String,
}

impl UiState {
    pub fn new(
        model_path: String,
        texture_path: String,
    ) -> Self {
        Self {
            show_panel: true,
            model_path,
            texture_path
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
                ui.heading("Version - 0.31\n");
                ui.heading(format!("Model name is '{}' ", self.model_path));
                ui.heading(format!("Texture name is '{}' \n", self.texture_path));
            });
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }
}