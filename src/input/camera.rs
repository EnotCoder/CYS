use winit_input_helper::WinitInputHelper;
use winit::keyboard::KeyCode;
use crate::constants::*;

pub fn handle_zoom(input: &WinitInputHelper, current: f32, zoom_step: f32) -> f32 {
    let scroll = input.scroll_diff();
    if scroll.1 > 0.0 && current < ZOOM_MAX {
        return (current + zoom_step).min(ZOOM_MAX);
    }
    if scroll.1 < 0.0 && current > ZOOM_MIN {
        return (current - zoom_step).max(ZOOM_MIN);
    }
    if input.key_pressed(KeyCode::KeyK) && current < ZOOM_MAX {
        return (current + zoom_step).min(ZOOM_MAX);
    }
    if input.key_pressed(KeyCode::KeyL) && current > ZOOM_MIN {
        return (current - zoom_step).max(ZOOM_MIN);
    }
    current
}
