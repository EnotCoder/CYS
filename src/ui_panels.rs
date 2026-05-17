// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
    pub model_path: String,
    pub texture_path: String,
    pub show_info_keyboard: bool
}

impl UiState {
    pub fn new(
        model_path: String,
        texture_path: String,
    ) -> Self {
        Self {
            show_panel: true,
            model_path,
            texture_path,
            show_info_keyboard: false,
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
            .show(ctx, |ui| {
                ui.heading("Version - 0.32\n");
                ui.heading(format!("Model name is '{}' ", self.model_path));
                ui.heading(format!("Texture name is '{}' \n", self.texture_path));

                if ui.button("Info keyboard").clicked() {
                    self.show_info_keyboard = !self.show_info_keyboard;
                }
            });

        if !self.show_info_keyboard {
            return;
        }

        Window::new("Info keyboard")
            .default_pos([580.0, 10.0])
            .default_size([280.0, 400.0])
            .resizable(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.heading("F1 - Hide windows");
            });
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }

}