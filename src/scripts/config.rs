// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Баланс игры (BalanceConfig): чтение scripts/config.lua через Lua.
//  Если файл отсутствует или повреждён — используются значения по умолчанию.
// ========================================================================

use std::collections::HashMap;
use std::path::Path;
use mlua::{Lua, Table, Value};

/// Путь к файлу баланса относительно корня проекта.
const CONFIG_PATH: &str = "scripts/config.lua";

/// Баланс игры, загружается один раз при старте из scripts/config.lua.
#[derive(Debug, Clone)]
pub struct BalanceConfig {
    // === Спавн покупателей ===
    pub shopper_spawn_interval: f64,
    pub shopper_spawn_cooldown: f64,
    pub max_shoppers: usize,
    // === Реген еды в box ===
    pub food_regen_tick: f64,
    pub food_regen_amount: i32,
    // === Цикл день/ночь ===
    pub day_cycle_secs: f64,
    pub day_secs: f64,
    pub night_start_secs: f64,
    pub night_secs: f64,
    pub fade_secs: f64,
    // === Движение/анимация NPC ===
    pub npc_speed: f32,
    pub walk_anim_interval: f64,
    pub npc_fade_speed: f32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    // === Покупатель ===
    pub cassa_wait_secs: f64,
    pub candy_wait_secs: f64,
    pub money_at_cassa: i32,
    pub money_at_candy: i32,
    pub candy_chance: f64,
    // === Вместимости хранилищ ===
    pub max_food_box: i32,
    pub max_food_rack: i32,
    pub max_food_candies: i32,
    pub candies_start_food: i32,
    // === Пороги смены текстур еды ===
    pub box_tex_threshold_1: i32,
    pub box_tex_threshold_2: i32,
    // === Экономика ===
    pub start_money: i32,
    // Аренда магазина: периодический расход, который нельзя отложить.
    pub rent_amount: i32,
    pub rent_interval_secs: f64,
    // Переопределения цен объектов по имени (что не упомянуто — берёт цену из данных объекта).
    pub object_prices: HashMap<String, i32>,
    // === Интерфейс ===
    pub font_path: String,
}

impl BalanceConfig {
    /// Загружает баланс из scripts/config.lua; при любой ошибке возвращает дефолты.
    pub fn load() -> Self {
        let mut cfg = BalanceConfig::default();
        // Если файла баланса нет — игра работает на дефолтных значениях.
        if !Path::new(CONFIG_PATH).exists() {
            eprintln!("[config] файл {CONFIG_PATH} не найден — используются дефолты");
            return cfg;
        }
        let source = match std::fs::read_to_string(CONFIG_PATH) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[config] ошибка чтения {CONFIG_PATH}: {e} — дефолты");
                return cfg;
            }
        };
        // Выполняем скрипт: он объявляет глобальную таблицу CONFIG с настройками.
        let lua = Lua::new();
        if let Err(e) = lua.load(&source).exec() {
            eprintln!("[config] ошибка выполнения {CONFIG_PATH}: {e} — дефолты");
            return cfg;
        }
        // Читаем таблицу CONFIG (все настройки лежат в ней); если её нет —
        // читаем top-level глобалы (обратная совместимость).
        let cfg_table = lua.globals().get::<Table>("CONFIG").ok();
        // Вспомогательный захват: читает ключ из CONFIG, при отсутствии — глобал.
        let get = |key: &str| -> Value {
            match cfg_table {
                Some(ref t) => {
                    if let Ok(v) = t.get::<Value>(key) {
                        if !matches!(v, Value::Nil) {
                            return v;
                        }
                    }
                    match lua.globals().get::<Value>(key) {
                        Ok(v) => v,
                        Err(_) => Value::Nil,
                    }
                }
                None => match lua.globals().get::<Value>(key) {
                    Ok(v) => v,
                    Err(_) => Value::Nil,
                },
            }
        };
        // Переопределения цен объектов: таблица name -> цена.
        if let Some(ref t) = cfg_table {
            if let Ok(prices) = t.get::<Table>("object_prices") {
                for pair in prices.pairs::<String, i64>() {
                    if let Ok((name, price)) = pair {
                        cfg.object_prices.insert(name, price as i32);
                    }
                }
            }
        }
        cfg.shopper_spawn_interval = get_f64(get("shopper_spawn_interval"), cfg.shopper_spawn_interval);
        cfg.shopper_spawn_cooldown = get_f64(get("shopper_spawn_cooldown"), cfg.shopper_spawn_cooldown);
        cfg.max_shoppers = get_i64(get("max_shoppers"), cfg.max_shoppers as i64).max(1) as usize;
        cfg.food_regen_tick = get_f64(get("food_regen_tick"), cfg.food_regen_tick);
        cfg.food_regen_amount = get_i64(get("food_regen_amount"), cfg.food_regen_amount as i64) as i32;
        cfg.day_cycle_secs = get_f64(get("day_cycle_secs"), cfg.day_cycle_secs);
        cfg.day_secs = get_f64(get("day_secs"), cfg.day_secs);
        cfg.night_start_secs = get_f64(get("night_start_secs"), cfg.night_start_secs);
        cfg.night_secs = get_f64(get("night_secs"), cfg.night_secs);
        cfg.fade_secs = get_f64(get("fade_secs"), cfg.fade_secs);
        cfg.npc_speed = get_f64(get("npc_speed"), cfg.npc_speed as f64) as f32;
        cfg.walk_anim_interval = get_f64(get("walk_anim_interval"), cfg.walk_anim_interval);
        cfg.npc_fade_speed = get_f64(get("npc_fade_speed"), cfg.npc_fade_speed as f64) as f32;
        cfg.spawn_x = get_i64(get("spawn_x"), cfg.spawn_x as i64) as i32;
        cfg.spawn_y = get_i64(get("spawn_y"), cfg.spawn_y as i64) as i32;
        cfg.cassa_wait_secs = get_f64(get("cassa_wait_secs"), cfg.cassa_wait_secs);
        cfg.candy_wait_secs = get_f64(get("candy_wait_secs"), cfg.candy_wait_secs);
        cfg.money_at_cassa = get_i64(get("money_at_cassa"), cfg.money_at_cassa as i64) as i32;
        cfg.money_at_candy = get_i64(get("money_at_candy"), cfg.money_at_candy as i64) as i32;
        cfg.candy_chance = get_f64(get("candy_chance"), cfg.candy_chance);
        cfg.max_food_box = get_i64(get("max_food_box"), cfg.max_food_box as i64) as i32;
        cfg.max_food_rack = get_i64(get("max_food_rack"), cfg.max_food_rack as i64) as i32;
        cfg.max_food_candies = get_i64(get("max_food_candies"), cfg.max_food_candies as i64) as i32;
        cfg.candies_start_food = get_i64(get("candies_start_food"), cfg.candies_start_food as i64) as i32;
        cfg.box_tex_threshold_1 = get_i64(get("box_tex_threshold_1"), cfg.box_tex_threshold_1 as i64) as i32;
        cfg.box_tex_threshold_2 = get_i64(get("box_tex_threshold_2"), cfg.box_tex_threshold_2 as i64) as i32;
        cfg.start_money = get_i64(get("start_money"), cfg.start_money as i64) as i32;
        cfg.rent_amount = get_i64(get("rent_amount"), cfg.rent_amount as i64) as i32;
        cfg.rent_interval_secs = get_f64(get("rent_interval_secs"), cfg.rent_interval_secs);
        cfg.font_path = get_string(get("font_path"), &cfg.font_path);
        cfg
    }

    /// Публикует CONFIG в глобалы Lua-скрипта (для npc.lua и будущих скриптов).
    pub fn publish_to_lua(&self, lua: &Lua) -> mlua::Result<()> {
        let t = lua.create_table()?;
        t.set("shopper_spawn_interval", self.shopper_spawn_interval)?;
        t.set("shopper_spawn_cooldown", self.shopper_spawn_cooldown)?;
        t.set("max_shoppers", self.max_shoppers)?;
        t.set("food_regen_tick", self.food_regen_tick)?;
        t.set("food_regen_amount", self.food_regen_amount)?;
        t.set("day_cycle_secs", self.day_cycle_secs)?;
        t.set("day_secs", self.day_secs)?;
        t.set("night_start_secs", self.night_start_secs)?;
        t.set("night_secs", self.night_secs)?;
        t.set("fade_secs", self.fade_secs)?;
        t.set("npc_speed", self.npc_speed)?;
        t.set("walk_anim_interval", self.walk_anim_interval)?;
        t.set("npc_fade_speed", self.npc_fade_speed)?;
        t.set("spawn_x", self.spawn_x)?;
        t.set("spawn_y", self.spawn_y)?;
        t.set("cassa_wait_secs", self.cassa_wait_secs)?;
        t.set("candy_wait_secs", self.candy_wait_secs)?;
        t.set("money_at_cassa", self.money_at_cassa)?;
        t.set("money_at_candy", self.money_at_candy)?;
        t.set("candy_chance", self.candy_chance)?;
        t.set("max_food_box", self.max_food_box)?;
        t.set("max_food_rack", self.max_food_rack)?;
        t.set("max_food_candies", self.max_food_candies)?;
        t.set("candies_start_food", self.candies_start_food)?;
        t.set("box_tex_threshold_1", self.box_tex_threshold_1)?;
        t.set("box_tex_threshold_2", self.box_tex_threshold_2)?;
        t.set("start_money", self.start_money)?;
        t.set("rent_amount", self.rent_amount)?;
        t.set("rent_interval_secs", self.rent_interval_secs)?;
        if !self.object_prices.is_empty() {
            let prices = lua.create_table()?;
            for (name, price) in &self.object_prices {
                prices.set(name.as_str(), *price)?;
            }
            t.set("object_prices", prices)?;
        }
        t.set("font_path", self.font_path.as_str())?;
        lua.globals().set("CONFIG", t)
    }
}

/// Достаёт число из Lua-значения (целое или дробное); иначе — дефолт.
fn get_f64(v: Value, default: f64) -> f64 {
    match v {
        Value::Integer(i) => i as f64,
        Value::Number(n) => n,
        _ => default,
    }
}

/// Достаёт целое из Lua-значения; иначе — дефолт.
fn get_i64(v: Value, default: i64) -> i64 {
    match v {
        Value::Integer(i) => i,
        Value::Number(n) => n as i64,
        _ => default,
    }
}

/// Достаёт строку из Lua-значения; иначе — дефолт.
fn get_string(v: Value, default: &str) -> String {
    match v {
        Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_else(|_| default.to_string()),
        _ => default.to_string(),
    }
}

impl Default for BalanceConfig {
    fn default() -> Self {
        BalanceConfig {
            shopper_spawn_interval: 3.0,
            shopper_spawn_cooldown: 2.0,
            max_shoppers: 15,
            food_regen_tick: 1.0,
            food_regen_amount: 1,
            day_cycle_secs: 120.0,
            day_secs: 55.0,
            night_start_secs: 60.0,
            night_secs: 115.0,
            fade_secs: 5.0,
            npc_speed: 3.0,
            walk_anim_interval: 0.3,
            npc_fade_speed: 2.0,
            spawn_x: 0,
            spawn_y: -3,
            cassa_wait_secs: 1.0,
            candy_wait_secs: 3.0,
            money_at_cassa: 5,
            money_at_candy: 1,
            candy_chance: 0.2,
            max_food_box: 20,
            max_food_rack: 15,
            max_food_candies: 10,
            candies_start_food: 10,
            box_tex_threshold_1: 8,
            box_tex_threshold_2: 12,
            start_money: 150,
            rent_amount: 15,
            rent_interval_secs: 60.0,
            object_prices: HashMap::new(),
            font_path: "font.otf".to_string(),
        }
    }
}
