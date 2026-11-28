// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  texture_pack — пользовательские замены текстур и анимаций.
//
//  Пак — папка с зеркалом assets/tex/ (textures/) и необязательным
//  pack.json. Все запросы к файлам текстур проходят через resolve(), поэтому
//  замена работает для карты, предмета, NPC и интерфейса одинаково.
// ========================================================================

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path};
use std::sync::{Mutex, OnceLock};

/// Мод-слои поверх активного пака задаёт игра: движок не знает о модах.
pub trait TextureLayerProvider: Send + Sync {
    fn layers(&self) -> Vec<String>;
    fn root(&self, mod_id: &str) -> String;
}

static LAYER_PROVIDER: OnceLock<Box<dyn TextureLayerProvider>> = OnceLock::new();

pub fn set_layer_provider(provider: Box<dyn TextureLayerProvider>) {
    let _ = LAYER_PROVIDER.set(provider);
}

fn mod_layers() -> Vec<String> {
    LAYER_PROVIDER
        .get()
        .map(|provider| provider.layers())
        .unwrap_or_default()
}

fn mod_root(mod_id: &str) -> String {
    LAYER_PROVIDER
        .get()
        .map(|provider| provider.root(mod_id))
        .unwrap_or_default()
}

/// Идентификатор ванильных текстур (пак не выбран).
pub const VANILLA_ID: &str = "";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ObjectOverride {
    pub texture: Option<String>,
    pub frame: Option<[i32; 2]>,
    pub atlas: Option<[i32; 2]>,
    pub animated: Option<bool>,
    pub frames: Option<Vec<String>>,
    pub fps: Option<f32>,
    #[serde(rename = "loop")]
    pub looping: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NpcOverride {
    pub idle: Option<Vec<String>>,
    pub walk: Option<Vec<String>>,
    pub fps: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PackManifest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    /// Логический путь ассета -> файл внутри пака.
    pub textures: HashMap<String, String>,
    pub objects: HashMap<String, ObjectOverride>,
    pub npcs: HashMap<String, NpcOverride>,
}

#[derive(Clone, Debug)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
}

#[derive(Default)]
struct PackState {
    available: Vec<PackInfo>,
    active: Option<String>,
    manifest: PackManifest,
}

static STATE: OnceLock<Mutex<PackState>> = OnceLock::new();

fn state() -> &'static Mutex<PackState> {
    STATE.get_or_init(|| Mutex::new(PackState::default()))
}

/// Корень папки паков: один и тот же путь на ПК и в APK-assets.
pub fn packs_root() -> &'static str {
    "assets/texture_pack"
}

fn pack_root(id: &str) -> String {
    format!("{}/{id}", packs_root())
}

/// Пути из манифеста не должны выходить за пределы пака.
pub fn safe_join(base: &str, relative: &str) -> Option<String> {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.starts_with('\\')
        || relative.contains('\\')
    {
        return None;
    }
    if Path::new(relative).components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(format!("{base}/{relative}"))
}

fn readable(path: &str) -> bool {
    crate::core::asset::load_bytes(path).is_ok()
}

fn read_manifest(id: &str) -> PackManifest {
    let Some(path) = safe_join(&pack_root(id), "pack.json") else {
        return PackManifest::default();
    };
    let Ok(text) = crate::core::asset::load_string(&path) else {
        return PackManifest::default();
    };
    match serde_json::from_str(&text) {
        Ok(manifest) => manifest,
        Err(e) => {
            eprintln!("Texture pack '{id}': invalid pack.json: {e}");
            PackManifest::default()
        }
    }
}

fn refresh_names(st: &mut PackState) {
    for pack in &mut st.available {
        let name = read_manifest(&pack.id)
            .name
            .unwrap_or_else(|| pack.id.clone());
        pack.name = name;
    }
}

/// Перечитывает список паков с диска. Добавление папки во время игры
/// становится видимым при открытии Settings.
pub fn rescan() {
    let mut available = Vec::new();
    for id in crate::core::asset::list_dir(packs_root()) {
        if id.starts_with('.') {
            continue;
        }
        let root = pack_root(&id);
        let is_pack = crate::core::asset::list_dir(&root)
            .iter()
            .any(|name| name == "textures" || name == "pack.json");
        if is_pack {
            available.push(PackInfo {
                name: id.clone(),
                id,
            });
        }
    }
    available.sort_by(|a, b| a.id.cmp(&b.id));

    let mut st = state().lock().unwrap();
    if let Some(id) = st.active.clone() {
        if !available.iter().any(|p| p.id == id) {
            st.active = None;
            st.manifest = PackManifest::default();
        }
    }
    st.available = available;
    refresh_names(&mut st);
}

/// Устанавливает активный пак. Неизвестный id откатывает игру на ваниль.
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
                .any(|p| p.id == *id)
        })
        .map(str::to_owned);
    let mut st = state().lock().unwrap();
    st.active = valid;
    st.manifest = st.active.as_deref().map(read_manifest).unwrap_or_default();
}

/// Применяет пак, выбранный в настройках игры.
pub fn init_with(id: Option<&str>) {
    set_active(id);
}

/// Список доступных паков, первый элемент — ваниль.
pub fn list() -> Vec<PackInfo> {
    let mut packs = vec![PackInfo {
        id: VANILLA_ID.to_string(),
        name: "Vanilla".to_string(),
    }];
    packs.extend(state().lock().unwrap().available.clone());
    packs
}

pub fn active_id() -> Option<String> {
    state().lock().unwrap().active.clone()
}

/// Разделяет виртуальный путь `tex.png@WxH` на файловый путь и размер.
fn split_sized(path: &str) -> (&str, Option<(f32, f32)>) {
    let Some(pos) = path.rfind('@') else {
        return (path, None);
    };
    let (base, tail) = path.split_at(pos);
    let mut parts = tail[1..].split('x');
    let (Some(w), Some(h), None) = (parts.next(), parts.next(), parts.next()) else {
        return (path, None);
    };
    let (Ok(w), Ok(h)) = (w.trim().parse::<f32>(), h.trim().parse::<f32>()) else {
        return (path, None);
    };
    if w > 0.0 && h > 0.0 {
        (base, Some((w, h)))
    } else {
        (path, None)
    }
}

/// Находит файл для логического пути текстуры. Суффикс `@WxH` не является
/// частью имени файла и отбрасывается. При любом сбое возвращается ванильный
/// путь — загрузчик сам подставит null.png, если и его нет.
pub fn resolve(path: &str) -> String {
    let (base, _) = split_sized(path);

    // Моды с текстурами — дополнительные слои поверх активного пака.
    // Работают и без выбранного пака. Позже загруженный мод перекрывает ранний.
    let mod_layers = mod_layers();
    if let Some(rel) = base.strip_prefix("assets/tex/") {
        for mod_id in mod_layers.iter().rev() {
            let mod_root = format!("{}/textures", mod_root(mod_id));
            if let Some(candidate) = safe_join(&mod_root, rel) {
                if readable(&candidate) {
                    return candidate;
                }
            }
        }
    }

    let st = state().lock().unwrap();
    let Some(id) = st.active.as_deref() else {
        return base.to_string();
    };
    let root = pack_root(id);

    if let Some(rel) = base.strip_prefix("assets/tex/") {
        if let Some(candidate) = safe_join(&format!("{root}/textures"), rel) {
            if readable(&candidate) {
                return candidate;
            }
        }
    }
    if let Some(target) = st.manifest.textures.get(base) {
        if let Some(candidate) = safe_join(&root, target) {
            if readable(&candidate) {
                return candidate;
            }
        }
    }
    base.to_string()
}

/// Проверка существования текстуры с учётом активного пака.
pub fn exists(path: &str) -> bool {
    readable(&resolve(path))
}

/// Версия правил резолва: меняется при смене пака или мод-слоёв.
pub fn resolution_generation() -> u64 {
    let layers = mod_layers();
    let st = state().lock().unwrap();
    let mut hasher = DefaultHasher::new();
    st.active.hash(&mut hasher);
    layers.hash(&mut hasher);
    hasher.finish()
}

static EXISTS_CACHE: OnceLock<Mutex<HashMap<(u64, String), bool>>> = OnceLock::new();

/// exists() с кэшем на время одной версии пака/модов.
pub fn exists_cached(path: &str, generation: u64) -> bool {
    let cache = EXISTS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (generation, path.to_string());
    if let Some(&value) = cache.lock().unwrap().get(&key) {
        return value;
    }
    let value = exists(path);
    cache.lock().unwrap().insert(key, value);
    value
}

/// Убирает кэш existence после полной смены ассетов.
pub fn clear_caches() {
    if let Some(cache) = EXISTS_CACHE.get() {
        cache.lock().unwrap().clear();
    }
}

fn object_override(name: &str) -> Option<ObjectOverride> {
    state().lock().unwrap().manifest.objects.get(name).cloned()
}

fn npc_override(id: &str) -> Option<NpcOverride> {
    state().lock().unwrap().manifest.npcs.get(id).cloned()
}

fn pack_relative_file(id: &str, target: &str) -> Option<String> {
    let candidate = safe_join(&pack_root(id), target)?;
    readable(&candidate).then_some(candidate)
}

fn with_active<T>(f: impl FnOnce(&str, &PackManifest) -> Option<T>) -> Option<T> {
    let st = state().lock().unwrap();
    let id = st.active.as_deref()?;
    f(id, &st.manifest)
}

// ========================================================================
//  Переопределения предметов каталога
// ========================================================================

pub fn object_texture(name: &str, default: &str) -> String {
    with_active(|id, manifest| {
        let target = manifest.objects.get(name)?.texture.as_deref()?;
        pack_relative_file(id, target)
    })
    .unwrap_or_else(|| default.to_string())
}

pub fn object_frame(name: &str, default: [i32; 2]) -> [i32; 2] {
    object_override(name)
        .and_then(|o| o.frame)
        .unwrap_or(default)
}

pub fn object_atlas(name: &str, default: [i32; 2]) -> [i32; 2] {
    let atlas = object_override(name)
        .and_then(|o| o.atlas)
        .unwrap_or(default);
    [atlas[0].max(1), atlas[1].max(1)]
}

pub fn object_animated(name: &str, default: bool) -> bool {
    object_override(name)
        .and_then(|o| o.animated)
        .unwrap_or(default)
}

pub fn object_frames(name: &str, default: &[&str]) -> Vec<String> {
    let Some(files) = object_override(name).and_then(|o| o.frames) else {
        return default.iter().map(|p| p.to_string()).collect();
    };
    let frames: Vec<String> = files
        .iter()
        .filter_map(|file| with_active(|id, _| pack_relative_file(id, file)))
        .collect();
    if frames.is_empty() {
        default.iter().map(|p| p.to_string()).collect()
    } else {
        frames
    }
}

pub fn object_fps(name: &str) -> f32 {
    object_override(name)
        .and_then(|o| o.fps)
        .unwrap_or(1.0)
        .max(0.01)
}

pub fn object_loop(name: &str) -> bool {
    object_override(name)
        .and_then(|o| o.looping)
        .unwrap_or(true)
}

// ========================================================================
//  Переопределения покупателей
// ========================================================================

/// Возвращает кадры idle, walk и скорость анимации для покупателя.
/// Незаданные списки остаются ванильными.
pub fn npc_textures(
    npc_id: &str,
    idle: &str,
    walk_1: &str,
    walk_2: &str,
) -> (Vec<String>, Vec<String>, Option<f32>) {
    let Some(ov) = npc_override(npc_id) else {
        return (
            vec![idle.to_string()],
            vec![walk_1.to_string(), walk_2.to_string()],
            None,
        );
    };
    let files = |list: Option<Vec<String>>, fallback: &[&str]| -> Vec<String> {
        let Some(files) = list else {
            return fallback.iter().map(|p| p.to_string()).collect();
        };
        let valid: Vec<String> = files
            .iter()
            .filter_map(|file| with_active(|id, _| pack_relative_file(id, file)))
            .collect();
        if valid.is_empty() {
            fallback.iter().map(|p| p.to_string()).collect()
        } else {
            valid
        }
    };
    (
        files(ov.idle, &[idle]),
        files(ov.walk, &[walk_1, walk_2]),
        ov.fps.map(|fps| fps.max(0.01)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_paths_must_not_escape_the_pack() {
        assert!(safe_join("texture_pack/neon", "textures/map/grass.png").is_some());
        assert!(safe_join("texture_pack/neon", "../assets/tex/map/grass.png").is_none());
        assert!(safe_join("texture_pack/neon", "/etc/passwd").is_none());
        assert!(safe_join("texture_pack/neon", "textures\\map\\grass.png").is_none());
        assert!(safe_join("texture_pack/neon", "").is_none());
    }

    #[test]
    fn sized_suffix_is_not_part_of_a_file_name() {
        assert_eq!(
            split_sized("assets/tex/a.png@2x1"),
            ("assets/tex/a.png", Some((2.0, 1.0)))
        );
        assert_eq!(
            split_sized("assets/tex/a@old.png@1x2"),
            ("assets/tex/a@old.png", Some((1.0, 2.0)))
        );
        assert_eq!(
            split_sized("assets/tex/a.png@0x1"),
            ("assets/tex/a.png@0x1", None)
        );
        assert_eq!(split_sized("assets/tex/a.png"), ("assets/tex/a.png", None));
    }

    #[test]
    fn manifest_accepts_loop_field_and_missing_fields() {
        let m: PackManifest = serde_json::from_str(
            r#"{"name":"Neon","objects":{"box":{"loop":false,"fps":4.0,"atlas":[2,2]}}}"#,
        )
        .unwrap();
        assert_eq!(m.name.as_deref(), Some("Neon"));
        let box_ov = &m.objects["box"];
        assert_eq!(box_ov.looping, Some(false));
        assert_eq!(box_ov.fps, Some(4.0));
        assert_eq!(box_ov.atlas, Some([2, 2]));
        assert_eq!(box_ov.texture, None);
    }
}
