// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  save — система миров (как в Minecraft)
// ========================================================================
//  Каждый мир хранится в отдельном файле worlds/world_{id}.json, а список
//  миров и их мета-данные — в worlds/manifest.json. Ввод имени мира
//  осуществляется через системную экранную клавиатуру (IME), поэтому
//  сохранение/загрузка привязаны не к Ctrl+S/Ctrl+L, а к выбору мира в
//  меню и автосохранению при выходе из игры.

use serde::{Serialize, Deserialize};

/// Мета-данные одного мира (для отображения в меню выбора).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldMeta {
    pub id: u32,
    pub name: String,
    pub created: i64,
    pub updated: i64,
}

/// Внутренний манифест: следующий свободный id и список миров.
#[derive(Serialize, Deserialize, Default)]
struct WorldManifest {
    next_id: u32,
    worlds: Vec<WorldMeta>,
}

const MANIFEST: &str = "worlds/manifest.json";

/// Путь к файлу сохранения мира относительно каталога сохранений
/// (дошивается к save_dir внутри asset::save_data/load_data).
pub fn world_save_path(id: u32) -> String {
    format!("worlds/world_{}.json", id)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn read_manifest() -> WorldManifest {
    match crate::core::asset::load_data(MANIFEST) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => WorldManifest::default(),
        },
        Err(_) => WorldManifest::default(),
    }
}

fn write_manifest(m: &WorldManifest) {
    if let Ok(json) = serde_json::to_string_pretty(m) {
        let _ = crate::core::asset::save_data(MANIFEST, json.as_bytes());
    }
}

/// Список существующих миров, отсортированный от недавно обновлённых к старым.
pub fn list_worlds() -> Vec<WorldMeta> {
    let mut worlds = read_manifest().worlds;
    worlds.sort_by(|a, b| b.updated.cmp(&a.updated));
    worlds
}

/// Ищет мета-данные мира по id (нужно для имени при загрузке).
pub fn world_meta(id: u32) -> Option<WorldMeta> {
    list_worlds().into_iter().find(|w| w.id == id)
}

/// Создаёт новый мир с заданным именем: резервирует уникальный id,
/// подбирает не совпадающее с другими мирами имя (при коллизии добавляет
/// суффикс "(N)", например "Мир" -> "Мир(1)"), обновляет манифест.
/// Файл сохранения появится при первом автосохранении.
pub fn create_world_with_name(desired: &str) -> WorldMeta {
    let mut m = read_manifest();
    let id = m.next_id;
    m.next_id += 1;
    let name = unique_world_name(&m.worlds, desired);
    let meta = WorldMeta {
        id,
        name,
        created: now(),
        updated: now(),
    };
    m.worlds.push(meta.clone());
    write_manifest(&m);
    meta
}

/// Подбирает уникальное имя: если базовое занято, добавляет "(1)", "(2)", ...
fn unique_world_name(worlds: &[WorldMeta], desired: &str) -> String {
    let base = desired.trim();
    let base = if base.is_empty() { "Мир" } else { base }.to_string();
    if !worlds.iter().any(|w| w.name == base) {
        return base;
    }
    let mut n = 1u32;
    loop {
        let cand = format!("{}({})", base, n);
        if !worlds.iter().any(|w| w.name == cand) {
            return cand;
        }
        n += 1;
    }
}

/// Создаёт новый мир с именем по умолчанию "Мир" (с дедупликацией).
#[allow(dead_code)]
pub fn create_world() -> WorldMeta {
    create_world_with_name("Мир")
}


/// Обновляет метку времени мира при сохранении.
pub fn touch_world(id: u32) {
    let mut m = read_manifest();
    if let Some(w) = m.worlds.iter_mut().find(|w| w.id == id) {
        w.updated = now();
    }
    write_manifest(&m);
}

/// Удаляет мир: убирает запись из манифеста и стирает файл сохранения.
/// Следующий id не переиспользуется (миры получают только возрастающие id).
pub fn delete_world(id: u32) {
    let mut m = read_manifest();
    m.worlds.retain(|w| w.id != id);
    write_manifest(&m);
    let _ = crate::core::asset::delete_data(&world_save_path(id));
}

/// Какой мир запустить при входе в игру. Выставляется меню перед Switch("game").
#[derive(Clone)]
pub enum WorldSelection {
    None,
    New(u32, String),
    Load(u32),
}

/// Глобальный выбор мира (аналог UI_UNIFORMS в init.rs): сцены не могут
/// передавать параметры через SceneAction, поэтому используем статику.
pub static SELECTED_WORLD: std::sync::Mutex<WorldSelection> =
    std::sync::Mutex::new(WorldSelection::None);
