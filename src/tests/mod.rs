// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  tests — модуль юнит-тестов крейта.
//  Подключается из src/main.rs директивой #[cfg(test)] mod tests;
//  сюда добавляются тесты по мере развития игры.
// ========================================================================

#[test]
fn mode_cycle_includes_move_mode() {
    use crate::core::constants::{MODE_BUILD, MODE_DELETE, MODE_INTERACT, MODE_MOVE};

    assert_eq!(crate::input::interact::next_mode(MODE_MOVE), MODE_INTERACT);
    assert_eq!(crate::input::interact::next_mode(MODE_INTERACT), MODE_BUILD);
    assert_eq!(crate::input::interact::next_mode(MODE_BUILD), MODE_DELETE);
    assert_eq!(crate::input::interact::next_mode(MODE_DELETE), MODE_MOVE);
}

#[test]
fn text_input_accepts_unicode_and_focus_request() {
    let input = crate::ui::text_input::TextInput::new();
    input.request_focus();
    assert!(input.is_active());
    assert!(input.take_focus_request());
    assert!(!input.take_focus_request());

    input.push("Привет, мир");
    assert_eq!(input.take(), "Привет, мир");

    input.set_active(false);
    assert!(!input.is_active());
}
