// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  platform.rs — абстракция источника ввода.
//  DesktopInput оборачивает WinitInputHelper (мышь + клавиатура).
//  TouchInput — заглушка под Android: тач-события эмулируют курсор и
//  левую кнопку мыши. Сцены и системы ввода работают с трейтом InputSource,
//  не зная, откуда пришёл ввод.
// ========================================================================

use winit::event::{DeviceEvent, MouseButton, WindowEvent};
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

#[cfg(target_os = "android")]
use winit::event::{Touch, TouchPhase};

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
    fn process_window_event(&mut self, event: &WindowEvent) { let _ = self.inner.process_window_event(event); }
    fn process_device_event(&mut self, event: &DeviceEvent) { let _ = self.inner.process_device_event(event); }
}

/// Мобильный (тач) ввод — заглушка.
/// Последнее касание эмулирует курсор и левую кнопку мыши; зум/клавиши
/// пока не реализованы (TODO: жесты пинча и экранные кнопки).
#[cfg(target_os = "android")]
pub struct TouchInput {
    pos: Option<(f32, f32)>,
    last_pos: Option<(f32, f32)>,
    pressed_this_frame: bool,
    held: bool,
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
            held: false,
        }
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
        matches!(btn, MouseButton::Left) && self.held
    }

    fn key_pressed(&self, _key: KeyCode) -> bool {
        false
    }

    fn key_held(&self, _key: KeyCode) -> bool {
        false
    }

    fn held_control(&self) -> bool {
        false
    }

    fn scroll_diff(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn close_requested(&self) -> bool {
        false
    }

    fn step(&mut self) {
        self.pressed_this_frame = false;
    }

    fn end_step(&mut self) {}

    fn process_window_event(&mut self, event: &WindowEvent) {
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
        match touch.phase {
            TouchPhase::Started => {
                self.pos = Some((x, y));
                self.last_pos = Some((x, y));
                self.pressed_this_frame = true;
                self.held = true;
            }
            TouchPhase::Moved => {
                self.pos = Some((x, y));
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.held = false;
            }
        }
    }
}
