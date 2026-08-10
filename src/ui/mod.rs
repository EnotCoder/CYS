// ========================================================================
//  UI: система интерфейса — компоненты (Panel/Button/Checkbox/Slider),
//  хотбар, настройки, FPS-счётчик и TextRenderer.
// ========================================================================

#![allow(dead_code)]

pub mod components;
pub mod fps;
pub mod settings;
pub mod system;
pub mod text_renderer;

pub use components::*;
pub use system::*;
