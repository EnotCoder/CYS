// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  shader_pack — пользовательские WGSL-шейдеры из assets/shaders/.
//
//  Активный шейдер заменяет встроенный shaders.wgsl при создании обоих
//  render-пайплайнов. Пак не меняет логику, только внешний вид спрайтов.
// ========================================================================

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

/// Идентификатор встроенного шейдера (пак не выбран).
pub const VANILLA_ID: &str = "";
pub const VANILLA_SOURCE: &str = include_str!("../gfx/shaders.wgsl");

const DEFAULT_FILE: &str = "shader.wgsl";
const MANIFEST_FILE: &str = "shader.json";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ShaderManifest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    /// WGSL-файл внутри папки шейдера.
    pub file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ShaderInfo {
    pub id: String,
    pub name: String,
}

#[derive(Default)]
struct ShaderState {
    available: Vec<ShaderInfo>,
    active: Option<String>,
    manifest: ShaderManifest,
    error: Option<String>,
}

static STATE: OnceLock<Mutex<ShaderState>> = OnceLock::new();

fn state() -> &'static Mutex<ShaderState> {
    STATE.get_or_init(|| Mutex::new(ShaderState::default()))
}

/// Корень папки шейдеров: один и тот же путь на ПК и в APK-assets.
pub fn shaders_root() -> &'static str {
    "assets/shaders"
}

fn shader_root(id: &str) -> String {
    format!("{}/{id}", shaders_root())
}

fn read_manifest(id: &str) -> Result<ShaderManifest, String> {
    let path = crate::core::texture_pack::safe_join(&shader_root(id), MANIFEST_FILE)
        .ok_or_else(|| "unsafe shader.json path".to_string())?;
    let Ok(text) = crate::core::asset::load_string(&path) else {
        return Ok(ShaderManifest::default());
    };
    serde_json::from_str(&text).map_err(|e| format!("shader '{id}': invalid shader.json: {e}"))
}

fn manifest_name(id: &str, manifest: &ShaderManifest) -> String {
    manifest
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| id.to_string())
}

fn read_source(id: &str, manifest: &ShaderManifest) -> Result<String, String> {
    let file = manifest
        .file
        .as_deref()
        .filter(|file| !file.is_empty())
        .unwrap_or(DEFAULT_FILE);
    let path = crate::core::texture_pack::safe_join(&shader_root(id), file)
        .ok_or_else(|| format!("shader '{id}': unsafe file path"))?;
    crate::core::asset::load_string(&path)
        .map_err(|e| format!("shader '{id}': cannot read {file}: {e}"))
}

/// Перечитывает список шейдеров с диска. Папка появляется в Settings сразу
/// после открытия окна, даже если игра уже запущена.
pub fn rescan() {
    let mut available = Vec::new();
    for id in crate::core::asset::list_dir(shaders_root()) {
        if id.starts_with('.') {
            continue;
        }
        let root = shader_root(&id);
        let is_shader = crate::core::asset::list_dir(&root)
            .iter()
            .any(|name| name == MANIFEST_FILE || name == DEFAULT_FILE);
        if !is_shader {
            continue;
        }
        let manifest = read_manifest(&id).unwrap_or_default();
        available.push(ShaderInfo {
            name: manifest_name(&id, &manifest),
            id,
        });
    }
    available.sort_by(|a, b| a.id.cmp(&b.id));

    let mut st = state().lock().unwrap();
    if let Some(id) = st.active.clone() {
        if !available.iter().any(|shader| shader.id == id) {
            st.active = None;
            st.manifest = ShaderManifest::default();
            st.error = None;
        }
    }
    st.available = available;
}

/// Устанавливает активный шейдер. Неизвестный id или битый манифест означают
/// возврат к встроенному шейдеру; причина ошибки сохраняется для Settings.
pub fn set_active(id: Option<&str>) {
    rescan();
    let valid = id
        .filter(|id| !id.is_empty())
        .filter(|id| {
            state()
                .lock()
                .unwrap()
                .available
                .iter()
                .any(|shader| shader.id == *id)
        })
        .map(str::to_owned);

    let mut st = state().lock().unwrap();
    st.active = None;
    st.manifest = ShaderManifest::default();
    st.error = None;
    let Some(id) = valid else { return };

    match read_manifest(&id) {
        Ok(manifest) => {
            st.active = Some(id);
            st.manifest = manifest;
        }
        Err(error) => st.error = Some(error),
    }
}

/// Применяет шейдер, выбранный в настройках игры.
pub fn init_with(id: Option<&str>) {
    set_active(id);
}

/// Список доступных шейдеров, первый элемент — Vanilla.
pub fn list() -> Vec<ShaderInfo> {
    let mut shaders = vec![ShaderInfo {
        id: VANILLA_ID.to_string(),
        name: "Vanilla".to_string(),
    }];
    shaders.extend(state().lock().unwrap().available.clone());
    shaders
}

pub fn active_id() -> Option<String> {
    state().lock().unwrap().active.clone()
}

pub fn error() -> Option<String> {
    state().lock().unwrap().error.clone()
}

pub fn set_error(message: String) {
    state().lock().unwrap().error = Some(message);
}

/// Возвращает WGSL активного шейдера. `None` означает встроенный шейдер;
/// ошибка чтения записывается в состояние и показывается в Settings.
pub fn source() -> Option<String> {
    let mut st = state().lock().unwrap();
    let Some(id) = st.active.clone() else {
        return None;
    };
    match read_source(&id, &st.manifest) {
        Ok(source) => {
            st.error = None;
            Some(source)
        }
        Err(error) => {
            st.error = Some(error);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_defaults_to_shader_wgsl() {
        let manifest: ShaderManifest =
            serde_json::from_str(r#"{"name":"CRT","author":"someone","version":"1.0.0"}"#).unwrap();
        assert_eq!(manifest.name.as_deref(), Some("CRT"));
        assert_eq!(manifest.file, None);
        assert_eq!(manifest.author.as_deref(), Some("someone"));
    }

    #[test]
    fn shader_files_must_stay_inside_the_pack() {
        let base = shader_root("pixel");
        assert!(crate::core::texture_pack::safe_join(&base, "shader.wgsl").is_some());
        assert!(crate::core::texture_pack::safe_join(&base, "../shaders.wgsl").is_none());
        assert!(crate::core::texture_pack::safe_join(&base, "/etc/passwd").is_none());
        assert!(crate::core::texture_pack::safe_join(&base, "sub\\shader.wgsl").is_none());
    }
}
