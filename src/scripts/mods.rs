// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  mods — загрузка модов игрока из папки mods/ и песочница Lua.
//
//  Мод меняет данные и логику: баланс, цены, каталог предметов, события и
//  необязательный слой текстур. Базовые config.lua / items.lua / npc.lua
//  загружаются первыми, затем моды в порядке из data/settings.json.
// ========================================================================

use mlua::{Function, Lua, Table, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::scripts::items::{ItemDef, TAB_NAMES};

pub const EVENT_STARTUP: &str = "startup";
pub const EVENT_TICK: &str = "tick";
pub const EVENT_NPC_UPDATE: &str = "npc_update";
pub const EVENT_OBJECT_PLACED: &str = "object_placed";
pub const EVENT_OBJECT_REMOVED: &str = "object_removed";
pub const EVENT_MONEY_CHANGED: &str = "money_changed";
pub const EVENT_DAY_START: &str = "day_start";
pub const EVENT_NIGHT_START: &str = "night_start";

pub const EVENTS: &[&str] = &[
    EVENT_STARTUP,
    EVENT_TICK,
    EVENT_NPC_UPDATE,
    EVENT_OBJECT_PLACED,
    EVENT_OBJECT_REMOVED,
    EVENT_MONEY_CHANGED,
    EVENT_DAY_START,
    EVENT_NIGHT_START,
];

const MANIFEST_FILE: &str = "mod.json";
const DEFAULT_ENTRY: &str = "main.lua";
const EVENT_TABLE: &str = "_CYS_MOD_EVENTS";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModManifest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub entry: Option<String>,
    pub load_after: Vec<String>,
    pub game_version: Option<String>,
    #[serde(default, rename = "use")]
    pub use_mod: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct ModInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub entry: String,
    pub load_after: Vec<String>,
    pub game_version: String,
    pub use_mod: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum ModValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
}

#[derive(Clone, Debug)]
pub enum ModArg {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Clone, Debug, Default)]
pub struct NpcEventResult {
    pub handled: bool,
    pub state: Option<i32>,
    pub timer: Option<f64>,
}

#[derive(Default)]
struct LoadContext {
    mod_id: String,
    root: String,
    balance: HashMap<String, ModValue>,
    prices: HashMap<String, i32>,
    items: Vec<ItemDef>,
    conflicts: Vec<String>,
}

struct ModState {
    available: Vec<ModInfo>,
    enabled: Vec<String>,
    loaded: Vec<String>,
    errors: HashMap<String, String>,
    conflicts: Vec<String>,
    balance: HashMap<String, ModValue>,
    prices: HashMap<String, i32>,
    items: Vec<ItemDef>,
    texture_mods: Vec<String>,
    lua: Option<Arc<Mutex<SharedLua>>>,
    initialized: bool,
}

/// mlua's `Lua` is !Send by default (it uses `Rc` internally). The game drives
/// every mod Lua call from the single-threaded winit event loop, and all access
/// to the VM is serialized by `ModState`'s mutex, so sharing it this way is safe.
struct SharedLua(Lua);

impl std::ops::Deref for SharedLua {
    type Target = Lua;

    fn deref(&self) -> &Lua {
        &self.0
    }
}

unsafe impl Send for SharedLua {}
unsafe impl Sync for SharedLua {}

static STATE: OnceLock<Mutex<ModState>> = OnceLock::new();
static LOAD: OnceLock<Mutex<LoadContext>> = OnceLock::new();

fn state() -> &'static Mutex<ModState> {
    STATE.get_or_init(|| {
        Mutex::new(ModState {
            available: Vec::new(),
            enabled: Vec::new(),
            loaded: Vec::new(),
            errors: HashMap::new(),
            conflicts: Vec::new(),
            balance: HashMap::new(),
            prices: HashMap::new(),
            items: Vec::new(),
            texture_mods: Vec::new(),
            lua: None,
            initialized: false,
        })
    })
}

fn load() -> &'static Mutex<LoadContext> {
    LOAD.get_or_init(|| Mutex::new(LoadContext::default()))
}

/// Корень модов: на Android они лежат внутри APK-assets.
pub fn mods_root() -> &'static str {
    if cfg!(target_os = "android") {
        "assets/mods"
    } else {
        "mods"
    }
}

pub fn mod_root(id: &str) -> String {
    format!("{}/{id}", mods_root())
}

fn read_manifest(id: &str) -> Result<ModManifest, String> {
    let path = crate::core::texture_pack::safe_join(&mod_root(id), MANIFEST_FILE)
        .ok_or_else(|| format!("mod '{id}': unsafe manifest path"))?;
    let text = crate::core::asset::load_string(&path)
        .map_err(|e| format!("mod '{id}': cannot read mod.json: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("mod '{id}': invalid mod.json: {e}"))
}

fn manifest_name(id: &str, manifest: &ModManifest) -> String {
    manifest
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| id.to_string())
}

fn info_from_manifest(id: &str, manifest: ModManifest) -> ModInfo {
    ModInfo {
        name: manifest_name(id, &manifest),
        description: manifest.description.unwrap_or_default(),
        author: manifest.author.unwrap_or_default(),
        version: manifest.version.unwrap_or_default(),
        id: id.to_string(),
        entry: manifest
            .entry
            .filter(|entry| !entry.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ENTRY.to_string()),
        load_after: manifest.load_after,
        game_version: manifest.game_version.unwrap_or_default(),
        use_mod: manifest.use_mod,
    }
}

/// Перечитывает список модов с диска. Папка появляется в окне Mods сразу
/// после открытия, даже если игра уже запущена.
pub fn rescan() -> Vec<ModInfo> {
    let mut found = Vec::new();
    for id in crate::core::asset::list_dir(mods_root()) {
        if id.starts_with('.') {
            continue;
        }
        let root = mod_root(&id);
        let has_manifest = crate::core::asset::list_dir(&root)
            .iter()
            .any(|name| name == MANIFEST_FILE);
        if !has_manifest {
            continue;
        }
        let info = match read_manifest(&id) {
            Ok(manifest) => info_from_manifest(&id, manifest),
            Err(error) => {
                eprintln!("[mods] {error}");
                info_from_manifest(&id, ModManifest::default())
            }
        };
        found.push(info);
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));

    let mut st = state().lock().unwrap();
    st.available = found.clone();
    st.enabled.retain(|id| found.iter().any(|m| m.id == *id));
    found
}

/// Стабильная топологическая сортировка: зависимости из `load_after`
/// идут раньше мода, остальной порядок сохраняется.
pub(crate) fn resolve_order(
    enabled: &[String],
    available: &[ModInfo],
) -> (Vec<String>, Vec<String>) {
    let known: Vec<&ModInfo> = enabled
        .iter()
        .filter_map(|id| available.iter().find(|m| m.id == *id))
        .collect();
    let known_ids: Vec<String> = known.iter().map(|m| m.id.clone()).collect();
    let mut ordered: Vec<String> = Vec::new();
    let mut visiting: Vec<String> = Vec::new();
    let mut conflicts = Vec::new();

    fn visit(
        id: &str,
        known: &[&ModInfo],
        known_ids: &[String],
        ordered: &mut Vec<String>,
        visiting: &mut Vec<String>,
        conflicts: &mut Vec<String>,
    ) {
        if ordered.iter().any(|x| x == id) || visiting.iter().any(|x| x == id) {
            return;
        }
        let Some(info) = known.iter().find(|m| m.id == id) else {
            return;
        };
        visiting.push(id.to_string());
        for dep in &info.load_after {
            let dep = dep.trim();
            if dep.is_empty() || dep == id {
                continue;
            }
            if !known_ids.iter().any(|x| x == dep) {
                conflicts.push(format!(
                    "'{}' depends on disabled or missing mod '{}'",
                    id, dep
                ));
                continue;
            }
            visit(dep, known, known_ids, ordered, visiting, conflicts);
        }
        visiting.retain(|x| x != id);
        if !ordered.iter().any(|x| x == id) {
            ordered.push(id.to_string());
        }
    }

    for id in &known_ids {
        visit(
            id,
            &known,
            &known_ids,
            &mut ordered,
            &mut visiting,
            &mut conflicts,
        );
    }
    for id in enabled {
        if !known_ids.iter().any(|x| x == id) {
            conflicts.push(format!("enabled mod '{}' was not found", id));
        }
    }
    (ordered, conflicts)
}

fn sandbox(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in [
        "io",
        "os",
        "require",
        "load",
        "loadstring",
        "package",
        "debug",
        "dofile",
        "loadfile",
        "coroutine",
        "print",
    ] {
        globals.set(name, Value::Nil)?;
    }
    Ok(())
}

fn lua_value_to_mod(value: Value) -> Option<ModValue> {
    match value {
        Value::Integer(i) => Some(ModValue::Int(i)),
        Value::Number(n) => Some(ModValue::Float(n)),
        Value::Boolean(b) => Some(ModValue::Bool(b)),
        Value::String(s) => s.to_str().ok().map(|s| ModValue::Str(s.to_string())),
        _ => None,
    }
}

fn mod_value_to_lua(lua: &Lua, value: &ModValue) -> mlua::Result<Value> {
    Ok(match value {
        ModValue::Int(i) => Value::Integer(*i),
        ModValue::Float(f) => Value::Number(*f),
        ModValue::Bool(b) => Value::Boolean(*b),
        ModValue::Str(s) => Value::String(lua.create_string(s)?),
    })
}

fn install_api(lua: &Lua) -> mlua::Result<()> {
    let api = lua.create_table()?;

    api.set(
        "log",
        lua.create_function(|_, msg: String| -> mlua::Result<()> {
            let id = load().lock().unwrap().mod_id.clone();
            println!("[mod:{}] {msg}", if id.is_empty() { "event" } else { &id });
            Ok(())
        })?,
    )?;

    let set_lua = lua.clone();
    api.set(
        "set_balance",
        lua.create_function(
            move |_, (key, value): (String, Value)| -> mlua::Result<()> {
                let Some(parsed) = lua_value_to_mod(value.clone()) else {
                    return Err(mlua::Error::RuntimeError(
                        "mod.set_balance expects a number, boolean or string".to_string(),
                    ));
                };
                load()
                    .lock()
                    .unwrap()
                    .balance
                    .insert(key.clone(), parsed.clone());
                let config: Table = set_lua.globals().get("CONFIG")?;
                config.set(key, mod_value_to_lua(&set_lua, &parsed)?)?;
                Ok(())
            },
        )?,
    )?;

    api.set(
        "get_balance",
        lua.create_function(|lua, key: String| -> mlua::Result<Value> {
            let config: Table = lua.globals().get("CONFIG")?;
            config.get(key)
        })?,
    )?;

    let price_lua = lua.clone();
    api.set(
        "set_price",
        lua.create_function(move |_, (name, price): (String, i64)| -> mlua::Result<()> {
            load()
                .lock()
                .unwrap()
                .prices
                .insert(name.clone(), price as i32);
            let config: Table = price_lua.globals().get("CONFIG")?;
            let prices: Table = config.get("object_prices")?;
            prices.set(name, price)?;
            Ok(())
        })?,
    )?;

    api.set(
        "register_item",
        lua.create_function(|_, def: Table| -> mlua::Result<()> {
            let item = item_from_lua(&def)?;
            let mut ctx = load().lock().unwrap();
            if let Some(index) = ctx
                .items
                .iter()
                .position(|existing| existing.name == item.name)
            {
                ctx.conflicts
                    .push(format!("item '{}' redefined (later mod wins)", item.name));
                ctx.items[index] = item;
            } else {
                ctx.items.push(item);
            }
            Ok(())
        })?,
    )?;

    let on_lua = lua.clone();
    api.set(
        "on",
        lua.create_function(
            move |_, (event, handler): (String, Function)| -> mlua::Result<()> {
                if !EVENTS.contains(&event.as_str()) {
                    let mut ctx = load().lock().unwrap();
                    ctx.conflicts
                        .push(format!("unknown event '{event}' ignored"));
                    return Ok(());
                }
                let events: Table = on_lua.globals().get(EVENT_TABLE)?;
                let list: Table = match events.get(event.as_str()) {
                    Ok(list) => list,
                    Err(_) => {
                        let list = on_lua.create_table()?;
                        events.set(event.as_str(), list.clone())?;
                        list
                    }
                };
                let entry = on_lua.create_table()?;
                entry.set("mod", load().lock().unwrap().mod_id.clone())?;
                entry.set("fn", handler)?;
                list.set(list.raw_len() + 1, entry)?;
                Ok(())
            },
        )?,
    )?;

    lua.globals().set("mod", api)?;
    lua.globals().set(EVENT_TABLE, lua.create_table()?)?;
    Ok(())
}

fn resolve_mod_file(root: &str, path: &str) -> String {
    if path.trim().is_empty() {
        return String::new();
    }
    crate::core::texture_pack::safe_join(root, path).unwrap_or_default()
}

fn item_from_lua(t: &Table) -> mlua::Result<ItemDef> {
    let value = |key: &str| t.get::<Value>(key).unwrap_or(Value::Nil);
    let string = |key: &str| -> Option<String> {
        match value(key) {
            Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
            _ => None,
        }
    };
    let integer = |key: &str, default: i64| -> i64 {
        match value(key) {
            Value::Integer(i) => i,
            Value::Number(n) => n as i64,
            _ => default,
        }
    };
    let number = |key: &str, default: f64| -> f64 {
        match value(key) {
            Value::Integer(i) => i as f64,
            Value::Number(n) => n,
            _ => default,
        }
    };
    let boolean = |key: &str, default: bool| -> bool {
        match value(key) {
            Value::Boolean(b) => b,
            _ => default,
        }
    };
    let pair = |key: &str, default: [i32; 2]| -> [i32; 2] {
        let Ok(p) = t.get::<Table>(key) else {
            return default;
        };
        [
            p.get::<i64>(1).unwrap_or(default[0] as i64) as i32,
            p.get::<i64>(2).unwrap_or(default[1] as i64) as i32,
        ]
    };
    let strings = |key: &str| -> Vec<String> {
        let Ok(list) = t.get::<Table>(key) else {
            return Vec::new();
        };
        list.sequence_values::<String>()
            .filter_map(Result::ok)
            .collect()
    };

    let name = string("name").ok_or_else(|| {
        mlua::Error::RuntimeError("mod.register_item requires a 'name' field".to_string())
    })?;
    let tab = string("tab").unwrap_or_else(|| TAB_NAMES[0].to_string());
    let root = load().lock().unwrap().root.clone();
    let texture = resolve_mod_file(&root, &string("texture").unwrap_or_default());
    let icon = {
        let requested = string("icon").unwrap_or_default();
        let resolved = resolve_mod_file(&root, &requested);
        if resolved.is_empty() {
            texture.clone()
        } else {
            resolved
        }
    };
    let frames: Vec<String> = strings("frames")
        .iter()
        .map(|path| resolve_mod_file(&root, path))
        .filter(|path| !path.is_empty())
        .collect();

    Ok(ItemDef {
        name,
        category: string("category").unwrap_or_else(|| category_from_tab(&tab)),
        width: integer("width", 1) as i32,
        height: integer("height", 1) as i32,
        texture,
        icon,
        icon_size: [
            number("icon_width", 1.0) as f32,
            number("icon_height", 1.0) as f32,
        ],
        price: integer("price", 0) as i32,
        frame: pair("frame", [0, 0]),
        atlas: pair("atlas", [1, 1]),
        animated: boolean("animated", false) || frames.len() >= 2,
        frames,
        flower: boolean("flower", false),
        tab,
    })
}

fn category_from_tab(tab: &str) -> String {
    match tab {
        "carpets" => "carpet",
        "walldecor" => "walldecor",
        "outdoor" => "outdoor",
        "light" => "light",
        _ => "regular",
    }
    .to_string()
}

fn load_base_scripts(lua: &Lua) {
    for (label, path) in [
        ("config", "assets/scripts/config.lua"),
        ("items", "assets/scripts/items.lua"),
        ("npc", "assets/scripts/npc.lua"),
    ] {
        match crate::core::asset::load_string(path) {
            Ok(source) => {
                if let Err(e) = lua.load(&source).set_name(path).exec() {
                    eprintln!("[mods] base {label} script failed: {e}");
                }
            }
            Err(_) => eprintln!("[mods] base {label} script not found: {path}"),
        }
    }
}

/// Загружает/base-скрипты и все включённые моды. Ошибка одного мода не
/// останавливает остальные: она сохраняется и показывается в окне Mods.
pub fn reload(enabled: &[String]) {
    let available = rescan();
    let (order, order_conflicts) = resolve_order(enabled, &available);

    let lua = Lua::new();
    if let Err(e) = sandbox(&lua) {
        eprintln!("[mods] sandbox setup failed: {e}");
        return;
    }
    if let Err(e) = install_api(&lua) {
        eprintln!("[mods] Lua API setup failed: {e}");
        return;
    }
    load_base_scripts(&lua);

    {
        let mut st = state().lock().unwrap();
        st.available = available;
        st.enabled = enabled.to_vec();
        st.loaded.clear();
        st.errors.clear();
        st.conflicts = order_conflicts;
        st.balance.clear();
        st.prices.clear();
        st.items.clear();
        st.texture_mods.clear();
        st.lua = Some(Arc::new(Mutex::new(SharedLua(lua.clone()))));
        st.initialized = true;
    }

    for id in &order {
        let Some(info) = state()
            .lock()
            .unwrap()
            .available
            .iter()
            .find(|m| m.id == *id)
            .cloned()
        else {
            continue;
        };
        let entry = crate::core::texture_pack::safe_join(&mod_root(id), &info.entry)
            .ok_or_else(|| format!("unsafe entry path: {}", info.entry));
        let source = entry.and_then(|path| {
            crate::core::asset::load_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", info.entry))
        });
        let source = match source {
            Ok(source) => source,
            Err(error) => {
                state().lock().unwrap().errors.insert(id.clone(), error);
                continue;
            }
        };
        {
            let mut ctx = load().lock().unwrap();
            ctx.mod_id = id.clone();
            ctx.root = mod_root(id);
            ctx.balance.clear();
            ctx.prices.clear();
            ctx.items.clear();
            ctx.conflicts.clear();
        }
        let result = lua
            .load(&source)
            .set_name(format!("{}/{id}/{}", mods_root(), info.entry))
            .exec();
        if let Err(error) = result {
            state()
                .lock()
                .unwrap()
                .errors
                .insert(id.clone(), format!("Lua error: {error}"));
            continue;
        }
        {
            let ctx = load().lock().unwrap();
            let mut st = state().lock().unwrap();
            st.balance.extend(ctx.balance.clone());
            st.prices.extend(ctx.prices.clone());
            for item in &ctx.items {
                if let Some(old) = st.items.iter_mut().find(|x| x.name == item.name) {
                    *old = item.clone();
                } else {
                    st.items.push(item.clone());
                }
            }
            st.conflicts.extend(ctx.conflicts.clone());
        }
        state().lock().unwrap().loaded.push(id.clone());
        emit(EVENT_STARTUP, &[("mod", ModArg::Str(id.clone()))], 0.0);
    }
    {
        let mut st = state().lock().unwrap();
        st.texture_mods = st
            .loaded
            .iter()
            .filter(|id| {
                !crate::core::asset::list_dir(&format!("{}/textures", mod_root(id))).is_empty()
            })
            .cloned()
            .collect();
    }
    load().lock().unwrap().mod_id.clear();
}

/// Итоговый список включённых модов: `use` из mod.json главнее старого списка
/// из settings.json (он остаётся запасным для модов без поля `use`).
/// Порядок из настроек сохраняется, новые моды добавляются в конце.
pub fn enabled_from(available: &[ModInfo], saved: &[String]) -> Vec<String> {
    let mut result: Vec<String> = saved
        .iter()
        .filter(|id| {
            available
                .iter()
                .any(|m| m.id == **id && m.use_mod.unwrap_or(true))
        })
        .cloned()
        .collect();
    for info in available {
        if info.use_mod == Some(true) && !result.iter().any(|id| id == &info.id) {
            result.push(info.id.clone());
        }
    }
    result
}

/// Применяет настройки при запуске приложения.
pub fn init() {
    let saved = crate::save::SETTINGS.lock().unwrap().mods.clone();
    let available = rescan();
    let enabled = enabled_from(&available, &saved);
    reload(&enabled);
}

pub fn is_initialized() -> bool {
    state().lock().unwrap().initialized
}

pub fn list() -> Vec<ModInfo> {
    state().lock().unwrap().available.clone()
}

pub fn enabled_ids() -> Vec<String> {
    state().lock().unwrap().enabled.clone()
}

pub fn loaded_ids() -> Vec<String> {
    state().lock().unwrap().loaded.clone()
}

pub fn errors() -> HashMap<String, String> {
    state().lock().unwrap().errors.clone()
}

pub fn conflicts() -> Vec<String> {
    state().lock().unwrap().conflicts.clone()
}

/// Моды с текстурами в порядке загрузки (первый — нижний слой).
pub fn texture_layers() -> Vec<String> {
    state().lock().unwrap().texture_mods.clone()
}

/// Включает/выключает моды и сразу перезагружает песочницу. Логические
/// изменения появятся в новом мире или после перезапуска.
pub fn set_enabled(enabled: Vec<String>) {
    {
        let mut st = state().lock().unwrap();
        st.enabled = enabled.clone();
    }
    reload(&enabled);
}

/// Записывает поле `use` в mod.json одного мода. На Android ассеты внутри APK
/// доступны только на чтение, поэтому там состояние остаётся в settings.json.
pub fn write_use_flag(id: &str, enabled: bool) -> Result<(), String> {
    if cfg!(target_os = "android") {
        return Ok(());
    }
    let path = format!("{}/{MANIFEST_FILE}", mod_root(id));
    let text =
        crate::core::asset::load_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("invalid {path}: {e}"))?;
    let Some(object) = value.as_object_mut() else {
        return Err(format!("{path}: mod.json must be an object"));
    };
    object.insert("use".to_string(), serde_json::Value::Bool(enabled));
    let pretty =
        serde_json::to_string_pretty(&value).map_err(|e| format!("cannot encode {path}: {e}"))?;
    let target = crate::core::asset::user_path(&path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(&target, format!("{pretty}\n"))
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    {
        let mut st = state().lock().unwrap();
        if let Some(info) = st.available.iter_mut().find(|m| m.id == id) {
            info.use_mod = Some(enabled);
        }
    }
    Ok(())
}

/// Сохраняет выбор в `use` каждого mod.json и в settings.json, затем
/// перезагружает песочницу с новым порядком загрузки.
pub fn persist_enabled(enabled: &[String]) {
    for info in list() {
        let active = enabled.iter().any(|id| id == &info.id);
        if let Err(error) = write_use_flag(&info.id, active) {
            eprintln!("[mods] {error}");
        }
    }
    {
        let mut settings = crate::save::SETTINGS.lock().unwrap();
        settings.mods = enabled.to_vec();
        crate::save::save_settings(&settings);
    }
    set_enabled(enabled.to_vec());
}

pub fn apply_balance(config: &mut crate::scripts::config::BalanceConfig) {
    let st = state().lock().unwrap();
    for (key, value) in &st.balance {
        match (key.as_str(), value) {
            ("shopper_spawn_interval", ModValue::Float(v)) => config.shopper_spawn_interval = *v,
            ("shopper_spawn_interval", ModValue::Int(v)) => {
                config.shopper_spawn_interval = *v as f64
            }
            ("shopper_spawn_cooldown", ModValue::Float(v)) => config.shopper_spawn_cooldown = *v,
            ("shopper_spawn_cooldown", ModValue::Int(v)) => {
                config.shopper_spawn_cooldown = *v as f64
            }
            ("max_shoppers", ModValue::Int(v)) => config.max_shoppers = (*v).max(1) as usize,
            ("food_regen_tick", ModValue::Float(v)) => config.food_regen_tick = *v,
            ("food_regen_tick", ModValue::Int(v)) => config.food_regen_tick = *v as f64,
            ("food_regen_amount", ModValue::Int(v)) => config.food_regen_amount = *v as i32,
            ("day_cycle_secs", ModValue::Float(v)) => config.day_cycle_secs = *v,
            ("day_cycle_secs", ModValue::Int(v)) => config.day_cycle_secs = *v as f64,
            ("day_secs", ModValue::Float(v)) => config.day_secs = *v,
            ("day_secs", ModValue::Int(v)) => config.day_secs = *v as f64,
            ("night_start_secs", ModValue::Float(v)) => config.night_start_secs = *v,
            ("night_start_secs", ModValue::Int(v)) => config.night_start_secs = *v as f64,
            ("night_secs", ModValue::Float(v)) => config.night_secs = *v,
            ("night_secs", ModValue::Int(v)) => config.night_secs = *v as f64,
            ("fade_secs", ModValue::Float(v)) => config.fade_secs = *v,
            ("fade_secs", ModValue::Int(v)) => config.fade_secs = *v as f64,
            ("npc_speed", ModValue::Float(v)) => config.npc_speed = *v as f32,
            ("npc_speed", ModValue::Int(v)) => config.npc_speed = *v as f32,
            ("walk_anim_interval", ModValue::Float(v)) => config.walk_anim_interval = *v,
            ("walk_anim_interval", ModValue::Int(v)) => config.walk_anim_interval = *v as f64,
            ("npc_fade_speed", ModValue::Float(v)) => config.npc_fade_speed = *v as f32,
            ("npc_fade_speed", ModValue::Int(v)) => config.npc_fade_speed = *v as f32,
            ("spawn_x", ModValue::Int(v)) => config.spawn_x = *v as i32,
            ("spawn_y", ModValue::Int(v)) => config.spawn_y = *v as i32,
            ("cassa_wait_secs", ModValue::Float(v)) => config.cassa_wait_secs = *v,
            ("cassa_wait_secs", ModValue::Int(v)) => config.cassa_wait_secs = *v as f64,
            ("candy_wait_secs", ModValue::Float(v)) => config.candy_wait_secs = *v,
            ("candy_wait_secs", ModValue::Int(v)) => config.candy_wait_secs = *v as f64,
            ("money_at_cassa", ModValue::Int(v)) => config.money_at_cassa = *v as i32,
            ("money_at_candy", ModValue::Int(v)) => config.money_at_candy = *v as i32,
            ("candy_chance", ModValue::Float(v)) => config.candy_chance = *v,
            ("candy_chance", ModValue::Int(v)) => config.candy_chance = *v as f64,
            ("max_food_box", ModValue::Int(v)) => config.max_food_box = *v as i32,
            ("max_food_rack", ModValue::Int(v)) => config.max_food_rack = *v as i32,
            ("max_food_candies", ModValue::Int(v)) => config.max_food_candies = *v as i32,
            ("candies_start_food", ModValue::Int(v)) => config.candies_start_food = *v as i32,
            ("box_tex_threshold_1", ModValue::Int(v)) => config.box_tex_threshold_1 = *v as i32,
            ("box_tex_threshold_2", ModValue::Int(v)) => config.box_tex_threshold_2 = *v as i32,
            ("start_money", ModValue::Int(v)) => config.start_money = *v as i32,
            ("rent_amount", ModValue::Int(v)) => config.rent_amount = *v as i32,
            ("rent_interval_secs", ModValue::Float(v)) => config.rent_interval_secs = *v,
            ("rent_interval_secs", ModValue::Int(v)) => config.rent_interval_secs = *v as f64,
            ("font_path", ModValue::Str(v)) => config.font_path = v.clone(),
            _ => {}
        }
    }
    for (name, price) in &st.prices {
        config.object_prices.insert(name.clone(), *price);
    }
}

pub fn item_defs() -> Vec<ItemDef> {
    state().lock().unwrap().items.clone()
}

fn arg_to_lua(lua: &Lua, value: &ModArg) -> mlua::Result<Value> {
    Ok(match value {
        ModArg::Str(s) => Value::String(lua.create_string(s)?),
        ModArg::Int(i) => Value::Integer(*i),
        ModArg::Float(f) => Value::Number(*f),
        ModArg::Bool(b) => Value::Boolean(*b),
    })
}

/// Вызывает обработчики события. Возвращает true, если мод пометил событие
/// обработанным (например, заменил поведение `npc_update`).
pub fn emit(event: &str, fields: &[(&str, ModArg)], dt: f64) -> bool {
    let lua_arc = {
        let st = state().lock().unwrap();
        if !st.initialized {
            return false;
        }
        match st.lua.clone() {
            Some(lua) => lua,
            None => return false,
        }
    };
    let lua = lua_arc.lock().unwrap();
    let events: Table = match lua.globals().get(EVENT_TABLE) {
        Ok(events) => events,
        Err(_) => return false,
    };
    let list: Table = match events.get(event) {
        Ok(list) => list,
        Err(_) => return false,
    };
    let data = match lua.create_table() {
        Ok(data) => data,
        Err(_) => return false,
    };
    for (key, value) in fields {
        if arg_to_lua(&lua, value)
            .and_then(|v| data.set(*key, v))
            .is_err()
        {
            return false;
        }
    }
    let _ = data.set("dt", dt);

    let mut handled = false;
    let mut failed: Vec<(String, String)> = Vec::new();
    let entries: Vec<Table> = list
        .sequence_values::<Table>()
        .filter_map(Result::ok)
        .collect();
    for entry in entries {
        let mod_id: String = entry.get("mod").unwrap_or_else(|_| "unknown".to_string());
        let handler: Function = match entry.get("fn") {
            Ok(handler) => handler,
            Err(_) => continue,
        };
        let result: mlua::Result<Option<Value>> = handler.call((data.clone(), dt));
        match result {
            Ok(Some(Value::Boolean(true))) => handled = true,
            Ok(Some(Value::Table(ret))) => {
                if ret.get::<bool>("handled").unwrap_or(false) {
                    handled = true;
                }
            }
            Ok(_) => {}
            Err(e) => failed.push((mod_id, format!("event '{event}': {e}"))),
        }
    }
    for (id, error) in failed {
        state().lock().unwrap().errors.insert(id, error);
    }
    handled
}

pub fn emit_npc_update(fields: &[(&str, ModArg)], dt: f64) -> NpcEventResult {
    let lua_arc = {
        let st = state().lock().unwrap();
        if !st.initialized {
            return NpcEventResult::default();
        }
        match st.lua.clone() {
            Some(lua) => lua,
            None => return NpcEventResult::default(),
        }
    };
    let lua = lua_arc.lock().unwrap();
    let events: Table = match lua.globals().get(EVENT_TABLE) {
        Ok(events) => events,
        Err(_) => return NpcEventResult::default(),
    };
    let list: Table = match events.get(EVENT_NPC_UPDATE) {
        Ok(list) => list,
        Err(_) => return NpcEventResult::default(),
    };
    let data = match lua.create_table() {
        Ok(data) => data,
        Err(_) => return NpcEventResult::default(),
    };
    for (key, value) in fields {
        if arg_to_lua(&lua, value)
            .and_then(|v| data.set(*key, v))
            .is_err()
        {
            return NpcEventResult::default();
        }
    }
    let _ = data.set("dt", dt);

    let mut result = NpcEventResult::default();
    let mut failed: Vec<(String, String)> = Vec::new();
    for entry in list.sequence_values::<Table>().filter_map(Result::ok) {
        let mod_id: String = entry.get("mod").unwrap_or_else(|_| "unknown".to_string());
        let handler: Function = match entry.get("fn") {
            Ok(handler) => handler,
            Err(_) => continue,
        };
        match handler.call::<Option<Value>>((data.clone(), dt)) {
            Ok(Some(Value::Boolean(true))) => result.handled = true,
            Ok(Some(Value::Table(ret))) => {
                if ret.get::<bool>("handled").unwrap_or(false) {
                    result.handled = true;
                }
            }
            Ok(_) => {}
            Err(e) => failed.push((mod_id, format!("event 'npc_update': {e}"))),
        }
    }
    result.state = data.get::<i32>("state").ok();
    result.timer = data.get::<f64>("timer").ok();
    for (id, error) in failed {
        state().lock().unwrap().errors.insert(id, error);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_defaults_to_main_lua() {
        let manifest: ModManifest = serde_json::from_str(
            r#"{"name":"Double Money","load_after":["base"],"game_version":"0.1.3"}"#,
        )
        .unwrap();
        assert_eq!(manifest.name.as_deref(), Some("Double Money"));
        assert_eq!(manifest.entry, None);
        assert_eq!(manifest.load_after, vec!["base".to_string()]);
        assert_eq!(manifest.game_version.as_deref(), Some("0.1.3"));
    }

    #[test]
    fn mod_paths_must_stay_inside_the_mod() {
        let base = mod_root("neon");
        assert!(crate::core::texture_pack::safe_join(&base, "main.lua").is_some());
        assert!(crate::core::texture_pack::safe_join(&base, "../config.lua").is_none());
        assert!(crate::core::texture_pack::safe_join(&base, "/etc/passwd").is_none());
        assert!(crate::core::texture_pack::safe_join(&base, "textures\\a.png").is_none());
    }

    #[test]
    fn load_after_reorders_enabled_mods() {
        let available = vec![
            info_from_manifest("a", ModManifest::default()),
            info_from_manifest(
                "b",
                ModManifest {
                    load_after: vec!["a".to_string()],
                    ..Default::default()
                },
            ),
        ];
        let enabled = vec!["b".to_string(), "a".to_string()];
        let (order, conflicts) = resolve_order(&enabled, &available);
        assert_eq!(order, vec!["a".to_string(), "b".to_string()]);
        assert!(conflicts.is_empty());
    }
}
