// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  items — чтение assets/scripts/items.lua (каталог предметов инвентаря).
//  Возвращает «сырые» описания; сборкой Object/ItemCatalog занимается
//  src/data/mod.rs. При ошибке возвращает None — используются Rust-дефолты.
// ========================================================================

use mlua::{Lua, Table, Value};

const ITEMS_PATH: &str = "assets/scripts/items.lua";

/// Порядок вкладок инвентаря. Совпадает с индексами Inventory::tab.
pub const TAB_NAMES: [&str; 6] = ["regular", "carpets", "walldecor", "outdoor", "light", "new_year"];

/// Один предмет, прочитанный из items.lua.
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub name: String,
    pub tab: String,
    pub width: i32,
    pub height: i32,
    pub texture: String,
    pub icon: String,
    pub icon_size: [f32; 2],
    pub price: i32,
    pub frame: [i32; 2],
    pub atlas: [i32; 2],
    pub animated: bool,
    pub frames: Vec<String>,
    pub flower: bool,
}

/// Читает ITEMS из items.lua. None — файла нет, он сломан или пуст.
pub fn load() -> Option<Vec<ItemDef>> {
    let source = match crate::core::asset::load_string(ITEMS_PATH) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[items] файл {ITEMS_PATH} не найден — встроенный каталог");
            return None;
        }
    };
    let lua = Lua::new();
    if let Err(e) = lua.load(&source).exec() {
        eprintln!("[items] ошибка выполнения {ITEMS_PATH}: {e} — встроенный каталог");
        return None;
    }
    let items = match lua.globals().get::<Table>("ITEMS") {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[items] в {ITEMS_PATH} нет таблицы ITEMS — встроенный каталог");
            return None;
        }
    };

    let mut defs = Vec::with_capacity(items.raw_len());
    for entry in items.sequence_values::<Table>() {
        let entry = match entry {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[items] пропущена запись: {e}");
                continue;
            }
        };
        let Some(name) = get_string(&entry, "name") else {
            eprintln!("[items] пропущена запись без name");
            continue;
        };
        let tab = get_string(&entry, "tab").unwrap_or_else(|| "regular".to_string());
        defs.push(ItemDef {
            name,
            tab,
            width: get_i64(&entry, "width", 1) as i32,
            height: get_i64(&entry, "height", 1) as i32,
            texture: get_string(&entry, "texture").unwrap_or_default(),
            icon: get_string(&entry, "icon").unwrap_or_default(),
            icon_size: [
                get_f64(&entry, "icon_width", 1.0) as f32,
                get_f64(&entry, "icon_height", 1.0) as f32,
            ],
            price: get_i64(&entry, "price", 0) as i32,
            frame: get_pair(&entry, "frame", [0, 0]),
            atlas: get_pair(&entry, "atlas", [1, 1]),
            animated: get_bool(&entry, "animated", false),
            frames: get_strings(&entry, "frames"),
            flower: get_bool(&entry, "flower", false),
        });
    }

    if defs.is_empty() {
        eprintln!("[items] {ITEMS_PATH} не содержит предметов — встроенный каталог");
        None
    } else {
        Some(defs)
    }
}

fn get_value(t: &Table, key: &str) -> Value {
    t.get::<Value>(key).unwrap_or(Value::Nil)
}

fn get_i64(t: &Table, key: &str, default: i64) -> i64 {
    match get_value(t, key) {
        Value::Integer(i) => i,
        Value::Number(n) => n as i64,
        _ => default,
    }
}

fn get_f64(t: &Table, key: &str, default: f64) -> f64 {
    match get_value(t, key) {
        Value::Integer(i) => i as f64,
        Value::Number(n) => n,
        _ => default,
    }
}

fn get_bool(t: &Table, key: &str, default: bool) -> bool {
    match get_value(t, key) {
        Value::Boolean(b) => b,
        _ => default,
    }
}

fn get_string(t: &Table, key: &str) -> Option<String> {
    match get_value(t, key) {
        Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
        _ => None,
    }
}

fn get_pair(t: &Table, key: &str, default: [i32; 2]) -> [i32; 2] {
    let Ok(pair) = t.get::<Table>(key) else { return default };
    [
        pair.get::<i64>(1).unwrap_or(default[0] as i64) as i32,
        pair.get::<i64>(2).unwrap_or(default[1] as i64) as i32,
    ]
}

fn get_strings(t: &Table, key: &str) -> Vec<String> {
    let Ok(list) = t.get::<Table>(key) else { return Vec::new() };
    list.sequence_values::<String>().filter_map(Result::ok).collect()
}
