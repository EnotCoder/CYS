// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// build.rs — встраивает иконку в Windows .exe (res/app.ico). На остальных
// платформах ничего не делает. Android-иконка задаётся через mipmap-ресурсы
// в Cargo.toml (package.metadata.android), здесь не обрабатывается.

fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("winres/app.ico");
        res.compile().expect("не удалось встроить иконку в .exe");
    }
}
