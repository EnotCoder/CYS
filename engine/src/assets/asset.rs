// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  asset.rs — кроссплатформенная загрузка игровых файлов.
//  На десктопе ассеты ищутся в CYS_ASSETS, рядом с исполняемым файлом,
//  в /usr/share/cys/assets и в CWD (для разработки). Пользовательские
//  данные (data/ и пользовательские копии assets/mods/, assets/shaders/,
//  assets/texture_pack/) пишутся в CYS_HOME, XDG_DATA_HOME/cys или
//  %APPDATA%/CYS.
//  На Android читает из APK-assets через AAssetManager, а сохранения
//  пишет в internal data dir приложения.
// ========================================================================

#[cfg(target_os = "android")]
use std::ffi::CString;
#[cfg(target_os = "android")]
use std::io::Read;
#[cfg(not(target_os = "android"))]
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

/// Глобальная ссылка на AndroidApp, устанавливается в android_main.
#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

#[cfg(not(target_os = "android"))]
static USER_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_app(app: AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

#[cfg(not(target_os = "android"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Корень исходного дерева: CWD или один из родителей Cargo-манифеста.
/// В workspace тесты и бинарник запускаются из каталога своего пакета.
#[cfg(not(target_os = "android"))]
fn source_root() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        starts.push(PathBuf::from(dir));
    }
    starts.into_iter().find_map(|start| {
        start.ancestors().find_map(|dir| {
            (dir.join("Cargo.toml").is_file() && dir.join("assets").join("tex").is_dir())
                .then(|| dir.to_path_buf())
        })
    })
}

/// Каталог пользовательских файлов. В исходном репозитории совпадает с корнем
/// workspace, чтобы `cargo run` и тесты работали с ./data и ./assets/....
#[cfg(not(target_os = "android"))]
fn user_root() -> &'static Path {
    USER_ROOT.get_or_init(|| {
        if let Some(dir) = std::env::var_os("CYS_HOME") {
            return PathBuf::from(dir);
        }
        if let Some(root) = source_root() {
            return root;
        }
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        #[cfg(not(target_os = "windows"))]
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|h| h.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(if cfg!(target_os = "windows") {
            "CYS"
        } else {
            "cys"
        })
    })
}

/// Путь внутри пользовательского каталога (для записи модов и настроек).
pub fn user_path(relative: &str) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let _ = relative;
        PathBuf::from("data")
    }
    #[cfg(not(target_os = "android"))]
    {
        user_root().join(relative)
    }
}

/// Каталоги, в которых лежат поставляемые ассеты без префикса `assets/`.
#[cfg(not(target_os = "android"))]
fn asset_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut push = |path: PathBuf| {
        if !path.as_os_str().is_empty() && !roots.contains(&path) {
            roots.push(path);
        }
    };
    if let Some(dir) = std::env::var_os("CYS_ASSETS") {
        let root = PathBuf::from(dir);
        let nested = root.join("assets");
        if nested.is_dir() {
            push(nested);
        }
        push(root);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            push(dir.join("assets"));
        }
    }
    push(PathBuf::from("/usr/share/cys/assets"));
    if let Ok(cwd) = std::env::current_dir() {
        push(cwd.join("assets"));
    }
    if let Some(root) = source_root() {
        push(root.join("assets"));
    }
    roots
}

/// Все кандидаты для чтения игрового файла, в порядке приоритета.
#[cfg(not(target_os = "android"))]
fn candidates(path: &str) -> Vec<PathBuf> {
    let mut result = vec![user_path(path)];
    let relative = path.strip_prefix("assets/").unwrap_or(path);
    for root in asset_roots() {
        let candidate = root.join(relative);
        if !result.contains(&candidate) {
            result.push(candidate);
        }
    }
    let legacy = PathBuf::from(path);
    if !result.contains(&legacy) {
        result.push(legacy);
    }
    result
}

/// Читает произвольный файл проекта (текстуру, карту, скрипт, шрифт, аудио).
pub fn load_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            // Android APK assets are already inside the "assets/" structure logically,
            // but the AAssetManager expects paths relative to the root of the assets folder.
            // If the path starts with "assets/", we strip it.
            let effective_path = if path.starts_with("assets/") {
                &path[7..]
            } else {
                path
            };
            let cpath = CString::new(effective_path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            if let Some(mut asset) = app.asset_manager().open(&cpath) {
                let mut buf = Vec::new();
                asset.read_to_end(&mut buf)?;
                return Ok(buf);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {effective_path} (original: {path})"),
            ));
        }
        std::fs::read(path)
    }
    #[cfg(not(target_os = "android"))]
    {
        let mut last_error = None;
        for candidate in candidates(path) {
            match std::fs::read(&candidate) {
                Ok(bytes) => return Ok(bytes),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {path}"),
            )
        }))
    }
}

/// Читает текстовый файл проекта (basement.txt, *.lua, конфиги).
pub fn load_string(path: &str) -> std::io::Result<String> {
    Ok(String::from_utf8_lossy(&load_bytes(path)?).into_owned())
}

/// Каталог для пользовательских сохранений (data/worlds, data/settings.json).
fn save_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            let base = app
                .internal_data_path()
                .unwrap_or_else(|| PathBuf::from("."));
            return base.join("data");
        }
    }
    user_path("data")
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

/// Удаляет пользовательские данные по имени файла (игнорирует отсутствие файла).
pub fn delete_data(name: &str) -> std::io::Result<()> {
    let path = save_dir().join(name);
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

/// Перечисляет имена файлов каталога. На десктопе объединяет пользовательские
/// и поставляемые папки (например, свои моды и моды из поставки).
pub fn list_dir(dir: &str) -> Vec<String> {
    #[cfg(target_os = "android")]
    {
        if let Some(app) = ANDROID_APP.get() {
            let effective_dir = if dir.starts_with("assets/") {
                &dir[7..]
            } else {
                dir
            };
            let cdir = match CString::new(effective_dir) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            if let Some(asset_dir) = app.asset_manager().open_dir(&cdir) {
                return asset_dir
                    .map(|cstr| cstr.to_string_lossy().into_owned())
                    .collect();
            }
            return Vec::new();
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
    #[cfg(not(target_os = "android"))]
    {
        let mut names = Vec::new();
        for candidate in candidates(dir) {
            if let Ok(entries) = std::fs::read_dir(candidate) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if !names.contains(&name) {
                            names.push(name);
                        }
                    }
                }
            }
        }
        names
    }
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    #[test]
    fn asset_candidates_cover_installed_and_source_layouts() {
        let list = candidates("assets/icon.png");
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                assert!(list.contains(&dir.join("assets").join("icon.png")));
            }
        }
        assert!(list.contains(&PathBuf::from("/usr/share/cys/assets/icon.png")));
        assert!(load_bytes("assets/icon.png").is_ok());
    }

    #[test]
    fn user_path_keeps_source_checkout_layout() {
        let root = source_root().expect("workspace root");
        assert_eq!(
            user_path("data/settings.json"),
            root.join("data/settings.json")
        );
    }
}
