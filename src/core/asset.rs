// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  asset.rs — кроссплатформенная загрузка игровых файлов.
//  На десктопе делегирует std::fs (пути относительно рабочей папки).
//  На Android читает из APK-assets через AAssetManager, а сохранения
//  пишет в internal data dir приложения.
// ========================================================================

#[cfg(target_os = "android")]
use std::io::Read;
use std::path::PathBuf;

#[cfg(target_os = "android")]
use std::sync::OnceLock;
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

/// Глобальная ссылка на AndroidApp, устанавливается в android_main.
#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_app(app: AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

/// Читает произвольный файл проекта (текстуру, карту, скрипт, шрифт, аудио).
pub fn load_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            let manager = app.asset_manager();
            if let Some(mut asset) = manager.open(path) {
                let mut buf = Vec::new();
                asset.read_to_end(&mut buf)?;
                return Ok(buf);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {path}"),
            ));
        }
    }
    std::fs::read(path)
}

/// Читает текстовый файл проекта (map.txt, *.lua, конфиги).
pub fn load_string(path: &str) -> std::io::Result<String> {
    Ok(String::from_utf8_lossy(&load_bytes(path)?).into_owned())
}

/// Каталог для пользовательских сохранений (save.json).
fn save_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            return app.internal_data_path();
        }
    }
    PathBuf::from(".")
}

/// Сохраняет пользовательские данные (например, save.json) по имени файла.
pub fn save_data(name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = save_dir().join(name);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
}

/// Загружает ранее сохранённые пользовательские данные по имени файла.
pub fn load_data(name: &str) -> std::io::Result<Vec<u8>> {
    std::fs::read(save_dir().join(name))
}

/// Перечисляет имена файлов внутри каталога (для директорий ассетов,
/// например sounds/). На десктопе — read_dir, на Android — AssetManager.list.
pub fn list_dir(dir: &str) -> Vec<String> {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            return app.asset_manager().list(dir).unwrap_or_default();
        }
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}
