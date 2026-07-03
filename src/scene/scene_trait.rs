// ========================================================================
//  Scene trait — общий интерфейс для всех сцен
// ========================================================================

pub enum SceneAction {
    Switch(String),
    Quit,
    VsyncToggle(bool),
    None,
}

pub trait Scene {
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::text_renderer::TextRenderer);
    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &winit_input_helper::WinitInputHelper, window_size: (f32, f32), text_renderer: &mut crate::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction;
    fn sprites(&self, ecs: &crate::EcsAdapter, visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>);
    fn map_size(&self) -> f32;
    fn camera_offset(&self) -> (f32, f32);
}
