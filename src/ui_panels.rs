// ui_panels.rs
use egui::{Context, Window, Slider, Button, Align2, ProgressBar};

pub struct UiState {
    pub show_panel: bool,
    pub show_info_keyboard: bool,
    pub show_effect: bool,
    pub use_texture: bool,  
    pub rotation_speed: f32,
}

impl UiState {
    pub fn new(
    ) -> Self {
        Self {
            show_panel: true,
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
            .title_bar(false)
            .show(ctx, |ui| {
                ui.heading("TMV Alpha");
            });
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }

}