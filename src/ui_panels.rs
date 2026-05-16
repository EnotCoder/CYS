// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
    pub model_position: [f32; 3],
    pub model_rotation: f32,
    pub camera_speed: f32,
    pub show_help: bool,
    pub fps: f32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_panel: true,
            model_position: [0.0, 0.0, 0.0],
            model_rotation: 0.0,
            camera_speed: 0.1,
            show_help: false,
            fps: 0.0,
        }
    }
    
    pub fn render(&mut self, ctx: &Context) {
        if !self.show_panel {
            return;
        }
        
        // Главное окно управления
        Window::new("🎮 Control Panel")
            .default_pos([10.0, 10.0])
            .default_size([280.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Model Controls");
            });
    }
}