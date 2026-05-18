// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
    pub model_path: String,
    pub texture_path: String,
    pub show_info_keyboard: bool,
    pub show_effect: bool,
    pub use_texture: bool,  
    pub rotation_speed: f32,
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
            show_effect: false,
            rotation_speed : 0.02,
            use_texture: true,
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
                ui.heading("Version - 0.36");

                ui.separator();

                ui.label(format!("Model path is '{}' ", self.model_path));
                ui.label(format!("Texture path is '{}' ", self.texture_path));

                ui.separator();
                
                if ui.button("Info keyboard").clicked() {
                    self.show_info_keyboard = !self.show_info_keyboard;
                }

                if ui.button("Effects").clicked() {
                    self.show_effect = !self.show_effect;
                }

                ui.separator();

                ui.label("Rotation Speed");
                ui.add(Slider::new(&mut self.rotation_speed, 0.0..=0.3));

                ui.separator();

                ui.checkbox(&mut self.use_texture, "Show_texture_model");
            });

        if self.show_info_keyboard {
            Window::new("Info keyboard")
                .default_pos([580.0, 10.0])
                .default_size([400.0, 400.0])
                .resizable(false)
                .movable(false)
                .show(ctx, |ui| {
                    ui.heading("F1 - Hide windows");
                });
        }

        if self.show_effect{
            Window::new("Effect")
                .default_pos([580.0, 10.0])
                .default_size([400.0, 400.0])
                .resizable(false)
                .movable(false)
                .show(ctx, |ui| {
                    ui.heading("Soon");
                });
        }
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }

}