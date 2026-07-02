use crate::EcsAdapter;
use crate::text_renderer::TextRenderer;
use crate::constants::*;
use winit_input_helper::WinitInputHelper;
use super::components::{Panel, Button, Checkbox};

// ========================================================================
//  Утилиты для проверки кликов (UI: map_size=1.0, cam_x=0, cam_y=0)
// ========================================================================

pub fn ndc_to_ui(mx: f32, my: f32, window_size: (f32, f32)) -> (f32, f32) {
    crate::util::ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0)
}

pub fn is_inside(wx: f32, wy: f32, cx: f32, cy: f32, half_w: f32, half_h: f32) -> bool {
    (wx - cx).abs() < half_w && (wy - cy).abs() < half_h
}

pub fn is_clicked(input: &WinitInputHelper, window_size: (f32, f32), cx: f32, cy: f32, half_w: f32, half_h: f32) -> bool {
    if !input.mouse_pressed(winit::event::MouseButton::Left) {
        return false;
    }
    let Some((mx, my)) = input.cursor() else { return false };
    let (wx, wy) = ndc_to_ui(mx, my, window_size);
    is_inside(wx, wy, cx, cy, half_w, half_h)
}

// ========================================================================
//  Panel
// ========================================================================

pub fn create_panel(ecs: &mut EcsAdapter, device: &wgpu::Device, queue: &wgpu::Queue, panel: &mut Panel) {
    destroy_panel(ecs, panel);
    let ent = ecs.add_ui_sized(panel.x, panel.y, panel.w, panel.h, "tex/black.png", device, queue);
    ecs.update_sprite_alpha(ent, panel.alpha);
    panel.entity = Some(ent);
}

pub fn destroy_panel(ecs: &mut EcsAdapter, panel: &mut Panel) {
    if let Some(ent) = panel.entity.take() {
        ecs.delete_entity(ent);
    }
}

// ========================================================================
//  Button
// ========================================================================

pub fn create_button(ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, btn: &mut Button) {
    destroy_button(ecs, btn);
    let bg = ecs.add_ui_sized(btn.x, btn.y, btn.w, btn.h, "tex/black.png", device, queue);
    let label = text_renderer.add_text(ecs, device, queue, &btn.label, btn.font_size, btn.x, btn.y, btn.w * 0.7, 1.0, WHITE);
    btn.bg = Some(bg);
    btn.text = Some(label);
}

pub fn destroy_button(ecs: &mut EcsAdapter, btn: &mut Button) {
    if let Some(ent) = btn.bg.take() {
        ecs.delete_entity(ent);
    }
    if let Some(ent) = btn.text.take() {
        ecs.delete_entity(ent);
    }
}

pub fn button_clicked(btn: &Button, input: &WinitInputHelper, window_size: (f32, f32)) -> bool {
    is_clicked(input, window_size, btn.x, btn.y, btn.w / 2.0, btn.h / 2.0)
}

// ========================================================================
//  Checkbox
// ========================================================================

const CHECKBOX_BOX_SIZE: f32 = 0.7;
const CHECKBOX_TEXT_WIDTH: f32 = 2.5;
const CHECKBOX_TEXT_GAP: f32 = 0.2;

pub fn create_checkbox(ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, checkbox: &mut Checkbox) {
    destroy_checkbox(ecs, checkbox);
    let tex = if checkbox.checked { "tex/ui/true.png" } else { "tex/ui/false.png" };
    let box_ent = ecs.add_ui_sized(checkbox.x, checkbox.y, CHECKBOX_BOX_SIZE, CHECKBOX_BOX_SIZE, tex, device, queue);
    let box_key = crate::util::sprite_cache_key("ui", checkbox.x, checkbox.y, tex, [0, 0], [1, 1], 1.0);
    checkbox.box_entity = Some(box_ent);
    checkbox.box_sprite_key = Some(box_key);
    let lx = checkbox.x + CHECKBOX_BOX_SIZE / 2.0 + CHECKBOX_TEXT_GAP + CHECKBOX_TEXT_WIDTH / 2.0;
    let label_ent = text_renderer.add_text(ecs, device, queue, &checkbox.label, checkbox.font_size, lx, checkbox.y, CHECKBOX_TEXT_WIDTH, 2.0, WHITE);
    let label_key = TextRenderer::sprite_cache_key(lx, checkbox.y, &checkbox.label, checkbox.font_size, 2.0, WHITE);
    checkbox.label_entity = Some(label_ent);
    checkbox.label_sprite_key = Some(label_key);
}

pub fn destroy_checkbox(ecs: &mut EcsAdapter, checkbox: &mut Checkbox) {
    if let Some(ent) = checkbox.box_entity.take() {
        ecs.delete_entity(ent);
    }
    if let Some(key) = checkbox.box_sprite_key.take() {
        ecs.sprite_cache.remove(&key);
    }
    if let Some(ent) = checkbox.label_entity.take() {
        ecs.delete_entity(ent);
    }
    if let Some(key) = checkbox.label_sprite_key.take() {
        ecs.sprite_cache.remove(&key);
    }
}

/// Пересоздаёт оба спрайта при изменении состояния
pub fn refresh_checkbox(ecs: &mut EcsAdapter, text_renderer: &mut TextRenderer, device: &wgpu::Device, queue: &wgpu::Queue, checkbox: &mut Checkbox) {
    create_checkbox(ecs, text_renderer, device, queue, checkbox);
}

pub fn checkbox_clicked(checkbox: &Checkbox, input: &WinitInputHelper, window_size: (f32, f32)) -> bool {
    let half = CHECKBOX_BOX_SIZE / 2.0 + 0.1;
    is_clicked(input, window_size, checkbox.x, checkbox.y, half, half)
}
