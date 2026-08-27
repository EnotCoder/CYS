// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  text_input — глобальный буфер ввода текста с экранной клавиатуры
//  (IME). Поле ввода «активно» только в состоянии назначения имени мира,
//  поэтому набор символов в игре не попадает в буфер. Backspace и Enter
//  передаются специальными маркерами ('\u{8}' и '\n').
// ========================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub struct TextInput {
    active: AtomicBool,
    buffer: Mutex<String>,
    /// Текущая IME-композиция (набираемый, но ещё не закоммиченный текст).
    /// На телефоне визуальная клавиатура держит набираемый текст именно здесь
    /// и часто не шлёт отдельного Commit, поэтому его тоже нужно учитывать.
    preedit: Mutex<String>,
}

impl TextInput {
    pub const fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            buffer: Mutex::new(String::new()),
            preedit: Mutex::new(String::new()),
        }
    }

    /// Включает/выключает захват ввода. При выключении буфер очищается.
    pub fn set_active(&self, on: bool) {
        if on {
            self.active.store(true, Ordering::SeqCst);
        } else {
            self.active.store(false, Ordering::SeqCst);
            self.buffer.lock().unwrap().clear();
            self.preedit.lock().unwrap().clear();
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Текущий набираемый (composition) текст IME.
    pub fn preedit(&self) -> String {
        self.preedit.lock().unwrap().clone()
    }

    /// Заменяет текущую IME-композицию.
    pub fn set_preedit(&self, s: &str) {
        *self.preedit.lock().unwrap() = s.to_string();
    }

    /// Очищает текущую IME-композицию.
    pub fn clear_preedit(&self) {
        self.preedit.lock().unwrap().clear();
    }

    /// Забирает текущую IME-композицию (возвращает и очищает).
    pub fn take_preedit(&self) -> String {
        std::mem::take(&mut *self.preedit.lock().unwrap())
    }

    /// Добавляет введённый текст (только если поле активно).
    pub fn push(&self, s: &str) {
        if !self.is_active() {
            return;
        }
        self.buffer.lock().unwrap().push_str(s);
    }

    /// Backspace — специальный маркер в буфере.
    pub fn push_backspace(&self) {
        if !self.is_active() {
            return;
        }
        self.buffer.lock().unwrap().push('\u{8}');
    }

    /// Enter / «Готово» — специальный маркер в буфере.
    pub fn push_enter(&self) {
        if !self.is_active() {
            return;
        }
        self.buffer.lock().unwrap().push('\n');
    }

    /// Забирает накопленный ввод и очищает буфер.
    pub fn take(&self) -> String {
        let mut b = self.buffer.lock().unwrap();
        std::mem::take(&mut *b)
    }
}

pub static TEXT_INPUT: TextInput = TextInput::new();

/// true, пока активна IME-композиция (набор через системную раскладку).
/// Пока он true, символы из KeyEvent.logical_key не дублируются (их отдаст
/// Ime::Commit), чтобы не было двойного ввода на ПК с включённой IME.
pub static IME_COMPOSING: AtomicBool = AtomicBool::new(false);
