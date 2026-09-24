// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  platform.rs — абстракция источника ввода.
//  DesktopInput оборачивает WinitInputHelper (мышь + клавиатура).
//  TouchInput — ввод для Android: один палец = тап/клик, перетаскивание
//  одним пальцем = движение камеры, щипок двумя пальцами = зум. Сцены и
//  системы ввода работают с трейтом InputSource, не зная, откуда пришёл ввод.
// ========================================================================

use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit_input_helper::WinitInputHelper;

#[cfg(target_os = "android")]
use std::collections::HashMap;
#[cfg(target_os = "android")]
use winit::event::{Touch, TouchPhase};

// Android soft keyboards can report deletion as a control character in text/IME events.
fn push_text_with_controls(s: &str) {
    for ch in s.chars() {
        match ch {
            '\u{8}' | '\u{7f}' => crate::ui::text_input::TEXT_INPUT.push_backspace(),
            '\n' | '\r' => crate::ui::text_input::TEXT_INPUT.push_enter(),
            c if !c.is_control() => {
                let text = c.to_string();
                crate::ui::text_input::TEXT_INPUT.push(&text);
            }
            _ => {}
        }
    }
}

fn process_ime_event(ime: &winit::event::Ime) {
    match ime {
        winit::event::Ime::Commit(s) => {
            crate::ui::text_input::IME_COMPOSING.store(false, std::sync::atomic::Ordering::SeqCst);
            let pre = crate::ui::text_input::TEXT_INPUT.take_preedit();
            let only_action = s.is_empty() || s.chars().all(|ch| ch == '\n' || ch == '\r');
            if only_action {
                push_text_with_controls(&pre);
                push_text_with_controls(s);
            } else {
                push_text_with_controls(s);
            }
        }
        winit::event::Ime::Preedit(s, _) => {
            crate::ui::text_input::IME_COMPOSING.store(!s.is_empty(), std::sync::atomic::Ordering::SeqCst);
            crate::ui::text_input::TEXT_INPUT.set_preedit(s);
        }
        winit::event::Ime::Enabled => {
            crate::ui::text_input::IME_COMPOSING.store(false, std::sync::atomic::Ordering::SeqCst);
            crate::ui::text_input::TEXT_INPUT.clear_preedit();
        }
        winit::event::Ime::Disabled => {
            crate::ui::text_input::IME_COMPOSING.store(false, std::sync::atomic::Ordering::SeqCst);
            let pre = crate::ui::text_input::TEXT_INPUT.take_preedit();
            push_text_with_controls(&pre);
        }
    }
}

fn process_keyboard_event(key: &KeyEvent) {
    if key.state != ElementState::Pressed {
        return;
    }

    let text = key.text.as_deref().or_else(|| match &key.logical_key {
        Key::Character(s) => Some(s.as_str()),
        _ => None,
    });
    let text_has_controls = text.map_or(false, |text| text.chars().any(|ch| ch.is_control()));
    let text_is_enter = text.map_or(false, |text| {
        text.contains('\n') || text.contains('\r')
    });
    let text_is_backspace = text.map_or(false, |text| {
        text.contains('\u{8}') || text.contains('\u{7f}')
    });
    let logical_enter = matches!(&key.logical_key, Key::Named(NamedKey::Enter));
    let is_enter = logical_enter
        || matches!(key.physical_key, PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter))
        || text_is_enter;
    let is_backspace = matches!(
        key.physical_key,
        PhysicalKey::Code(KeyCode::Backspace | KeyCode::NumpadBackspace | KeyCode::Delete)
    ) || matches!(
        &key.logical_key,
        Key::Named(NamedKey::Backspace | NamedKey::Delete)
    ) || text_is_backspace;

    if text_has_controls {
        if crate::ui::text_input::IME_COMPOSING.load(std::sync::atomic::Ordering::SeqCst) {
            let controls: String = text
                .unwrap_or("")
                .chars()
                .filter(|ch| matches!(ch, '\u{8}' | '\u{7f}' | '\n' | '\r'))
                .collect();
            push_text_with_controls(&controls);
        } else if let Some(text) = text {
            push_text_with_controls(text);
        }
        return;
    }

    if is_backspace {
        crate::ui::text_input::TEXT_INPUT.push_backspace();
    }
    if is_enter {
        crate::ui::text_input::TEXT_INPUT.push_enter();
    }

    if crate::ui::text_input::IME_COMPOSING.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }

    if let Some(text) = text {
        crate::ui::text_input::TEXT_INPUT.push(text);
    }
}

/// Единый интерфейс ввода для десктопа и мобильных платформ.
pub trait InputSource {
    fn cursor(&self) -> Option<(f32, f32)>;
    fn cursor_diff(&self) -> (f32, f32);
    fn mouse_pressed(&self, btn: MouseButton) -> bool;
    fn mouse_held(&self, btn: MouseButton) -> bool;
    fn key_pressed(&self, key: KeyCode) -> bool;
    fn key_held(&self, key: KeyCode) -> bool;
    fn held_control(&self) -> bool;
    fn scroll_diff(&self) -> (f32, f32);
    fn close_requested(&self) -> bool;
    fn step(&mut self);
    fn end_step(&mut self);
    fn process_window_event(&mut self, event: &WindowEvent);
    fn process_device_event(&mut self, event: &DeviceEvent);
}

/// Десктопный ввод: просто переадресует вызовы WinitInputHelper.
pub struct DesktopInput {
    inner: WinitInputHelper,
}

impl Default for DesktopInput {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopInput {
    pub fn new() -> Self {
        Self { inner: WinitInputHelper::new() }
    }
}

impl InputSource for DesktopInput {
    fn cursor(&self) -> Option<(f32, f32)> { self.inner.cursor() }
    fn cursor_diff(&self) -> (f32, f32) { self.inner.cursor_diff() }
    fn mouse_pressed(&self, btn: MouseButton) -> bool { self.inner.mouse_pressed(btn) }
    fn mouse_held(&self, btn: MouseButton) -> bool { self.inner.mouse_held(btn) }
    fn key_pressed(&self, key: KeyCode) -> bool { self.inner.key_pressed(key) }
    fn key_held(&self, key: KeyCode) -> bool { self.inner.key_held(key) }
    fn held_control(&self) -> bool { self.inner.held_control() }
    fn scroll_diff(&self) -> (f32, f32) { self.inner.scroll_diff() }
    fn close_requested(&self) -> bool { self.inner.close_requested() }
    fn step(&mut self) { let _ = self.inner.step(); }
    fn end_step(&mut self) { self.inner.end_step(); }
    fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Ime(ime) => process_ime_event(ime),
            WindowEvent::KeyboardInput { event: key, .. } => process_keyboard_event(key),
            _ => {}
        }
        let _ = self.inner.process_window_event(event);
    }
    fn process_device_event(&mut self, event: &DeviceEvent) { let _ = self.inner.process_device_event(event); }
}

/// Мобильный (тач) ввод.
/// - Один палец без движения = тап (эмулирует ЛКМ).
/// - Перетаскивание одним пальцем = движение камеры (эмулирует среднюю кнопку).
/// - Щипок двумя пальцами = зум (эмулирует колесо мыши).
#[cfg(target_os = "android")]
pub struct TouchInput {
    pos: Option<(f32, f32)>,
    last_pos: Option<(f32, f32)>,
    pressed_this_frame: bool,
    back_pressed: bool,
    tap_candidate: bool,
    dragging: bool,
    // Активные касания по id -> позиция (для распознавания щипка и пана).
    touches: HashMap<u64, (f32, f32)>,
    // Дистанция между двумя пальцами на предыдущем кадре для расчёта зума.
    pinch_last_dist: f32,
}

#[cfg(target_os = "android")]
impl Default for TouchInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "android")]
impl TouchInput {
    pub fn new() -> Self {
        Self {
            pos: None,
            last_pos: None,
            pressed_this_frame: false,
            back_pressed: false,
            tap_candidate: false,
            dragging: false,
            touches: HashMap::new(),
            pinch_last_dist: 0.0,
        }
    }

    fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (a.0 - b.0, a.1 - b.1);
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(target_os = "android")]
impl InputSource for TouchInput {
    fn cursor(&self) -> Option<(f32, f32)> {
        self.pos
    }

    fn cursor_diff(&self) -> (f32, f32) {
        match (self.last_pos, self.pos) {
            (Some((lx, ly)), Some((x, y))) => (x - lx, y - ly),
            _ => (0.0, 0.0),
        }
    }

    fn mouse_pressed(&self, btn: MouseButton) -> bool {
        matches!(btn, MouseButton::Left) && self.pressed_this_frame
    }

    fn mouse_held(&self, btn: MouseButton) -> bool {
        match btn {
            // ЛКМ «зажата» пока идёт тап или перетаскивание (для совместимости).
            MouseButton::Left => self.pressed_this_frame || self.dragging,
            // Средняя кнопка — именно она крутит камеру в update_camera.
            MouseButton::Middle => self.dragging,
            _ => false,
        }
    }

    fn key_pressed(&self, key: KeyCode) -> bool {
        self.back_pressed && matches!(key, KeyCode::Escape | KeyCode::BrowserBack)
    }

    fn key_held(&self, _key: KeyCode) -> bool {
        false
    }

    fn held_control(&self) -> bool {
        false
    }

    fn scroll_diff(&self) -> (f32, f32) {
        // Зум: изменение расстояния между пальцами. Пальцы разводятся —
        // расстояние растёт (scroll.1 > 0 → приближение в handle_zoom).
        if self.touches.len() >= 2 {
            let pts: Vec<(f32, f32)> = self.touches.values().copied().collect();
            let dist = Self::distance(pts[0], pts[1]);
            return (0.0, dist - self.pinch_last_dist);
        }
        (0.0, 0.0)
    }

    fn close_requested(&self) -> bool {
        false
    }

    fn step(&mut self) {
        self.pressed_this_frame = false;
        self.back_pressed = false;
    }

    fn end_step(&mut self) {
        // Фиксируем опорные точки для расчёта дельты следующего кадра.
        self.last_pos = self.pos;
        if self.touches.len() >= 2 {
            let pts: Vec<(f32, f32)> = self.touches.values().copied().collect();
            self.pinch_last_dist = Self::distance(pts[0], pts[1]);
        }
    }

    fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Ime(ime) => process_ime_event(ime),
            WindowEvent::KeyboardInput { event: key, .. } => {
                process_keyboard_event(key);
                if key.state == ElementState::Pressed
                    && (matches!(&key.logical_key, Key::Named(NamedKey::Escape | NamedKey::BrowserBack))
                        || matches!(key.physical_key, PhysicalKey::Code(KeyCode::Escape | KeyCode::BrowserBack)))
                {
                    self.back_pressed = true;
                }
            }
            _ => {}
        }
        if let WindowEvent::Touch(touch) = event {
            self.apply_touch(touch);
        }
    }

    fn process_device_event(&mut self, _event: &DeviceEvent) {}
}

#[cfg(target_os = "android")]
impl TouchInput {
    fn apply_touch(&mut self, touch: &Touch) {
        let (x, y) = (touch.location.x as f32, touch.location.y as f32);
        let id = touch.id;
        match touch.phase {
            TouchPhase::Started => {
                self.touches.insert(id, (x, y));
                if self.touches.len() == 1 {
                    self.pos = Some((x, y));
                    self.last_pos = Some((x, y));
                    self.tap_candidate = true;
                    self.dragging = false;
                    self.pressed_this_frame = false;
                } else if self.touches.len() == 2 {
                    self.tap_candidate = false;
                    self.dragging = false;
                    let pts: Vec<(f32, f32)> = self.touches.values().copied().collect();
                    self.pinch_last_dist = Self::distance(pts[0], pts[1]);
                }
            }
            TouchPhase::Moved => {
                if let Some(p) = self.touches.get_mut(&id) {
                    *p = (x, y);
                }
                if self.touches.len() == 1 {
                    if let Some((cx, cy)) = self.pos {
                        if Self::distance((cx, cy), (x, y)) > 8.0 {
                            self.tap_candidate = false;
                            self.dragging = true;
                        }
                    }
                    self.pos = Some((x, y));
                }
            }
            TouchPhase::Ended => {
                let was_tap = self.tap_candidate && !self.dragging;
                self.touches.remove(&id);
                if self.touches.is_empty() {
                    self.pressed_this_frame = was_tap;
                    self.tap_candidate = false;
                    self.dragging = false;
                } else if self.touches.len() == 1 {
                    let (_, p) = self.touches.iter().next().unwrap();
                    self.pos = Some(*p);
                    self.last_pos = Some(*p);
                    self.tap_candidate = false;
                    self.dragging = false;
                }
            }
            TouchPhase::Cancelled => {
                self.touches.remove(&id);
                if self.touches.is_empty() {
                    self.pressed_this_frame = false;
                    self.tap_candidate = false;
                    self.dragging = false;
                }
            }
        }
    }
}
