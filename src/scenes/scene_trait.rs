// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Scene trait — общий интерфейс для всех сцен
// ========================================================================

/// Действие, которое сцена возвращает из update() в главный цикл:
/// переключить сцену по имени, выйти из игры, переключить vsync
pub enum SceneAction {
    Switch(String),
    Quit,
    VsyncToggle(bool),
    None,
}

pub trait Scene {
    /// Вызывается один раз при входе в сцену (после очистки мира)
    fn on_enter(&mut self, ecs: &mut crate::EcsAdapter, text_renderer: &mut crate::ui::text_renderer::TextRenderer);
    /// Вызывается перед выходом из сцены (переключение на другую сцену или
    /// закрытие приложения). По умолчанию ничего не делает; GameScene здесь
    /// автосохраняет текущий мир.
    fn on_exit(&mut self, _ecs: &mut crate::EcsAdapter, _text_renderer: &mut crate::ui::text_renderer::TextRenderer) {}
    /// Ежекадровое обновление сцены: ввод, игровая логика; возвращает SceneAction
    fn update(&mut self, ecs: &mut crate::EcsAdapter, input: &dyn crate::input::platform::InputSource, window_size: (f32, f32), text_renderer: &mut crate::ui::text_renderer::TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneAction;
    /// Собирает спрайты всех слоёв рендера (земля/ковры/свет/декор/NPC/курсор/UI и т.д.);
    /// visible_bounds — прямоугольник видимой области для отсечения
    fn sprites(&self, ecs: &crate::EcsAdapter, visible_bounds: Option<(f32, f32, f32, f32)>) -> (Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>, Vec<crate::SpriteRenderData>);
    /// Размер карты сцены (масштаб камеры)
    fn map_size(&self) -> f32;
    /// Масштаб UI-слоя (1.0 — номинал, меньше на узких экранах), чтобы
    /// интерфейс помещался при любом соотношении сторон. По умолчанию 1.0.
    fn ui_size(&self) -> f32 { 1.0 }
    /// Смещение камеры сцены
    fn camera_offset(&self) -> (f32, f32);
    /// Коэффициент затемнения ночью 0..1 (по умолчанию 0 — день)
    fn night_factor(&self) -> f32 { 0.0 }
    /// Список источников света в сцене
    fn lights(&self, _ecs: &crate::EcsAdapter) -> Vec<crate::core::buffers::LightData> { Vec::new() }
}
