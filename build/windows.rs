// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Вызывается только при TARGET_OS=windows. Для кросс-сборки нужен
// windres из mingw-w64 (x86_64-w64-mingw32-windres).

pub fn embed_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("build/app.ico");
    if !cfg!(target_os = "windows") {
        let tool = match std::env::var("TARGET").as_deref() {
            Ok("x86_64-pc-windows-gnu") => "x86_64-w64-mingw32-windres",
            Ok("i686-pc-windows-gnu") => "i686-w64-mingw32-windres",
            _ => "windres",
        };
        res.set_windres_path(tool);
    }
    res.compile().unwrap_or_else(|e| {
        panic!(
            "не удалось встроить иконку build/app.ico в .exe: {e}. \
             Для кросс-сборки установите mingw-w64 (windres)"
        )
    });
}
