// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Точка входа build-скрипта (Cargo: package.build = "build/main.rs").
// Скрипт компилируется под ХОСТ, поэтому цель сборки определяем по
// CARGO_CFG_TARGET_OS, а не через cfg(target_os): иначе при кросс-сборке
// с Linux под mingw иконка молча не встраивается.

mod windows;

fn main() {
    println!("cargo:rerun-if-changed=build");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    windows::embed_icon();
}
