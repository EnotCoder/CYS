// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Вызывается только при TARGET_OS=windows. Для кросс-сборки нужен
// windres из mingw-w64 (x86_64-w64-mingw32-windres).

pub fn embed_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("build/app.ico");
    res.compile().unwrap_or_else(|e| {
        panic!(
            "не удалось встроить иконку build/app.ico в .exe: {e}. \
             Для кросс-сборки установите mingw-w64 (windres)"
        )
    });
}
