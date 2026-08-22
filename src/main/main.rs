// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Десктопная точка входа: делегирует в библиотеку `cys` (crate-type cdylib+rlib),
// которая содержит всю логику приложения и android_main для Android-сборки.
fn main() {
    cys::run();
}
