// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  data_integrity — инварианты констант, каталога объектов, баланса
//  и чистых функций данных (день/ночь, кэш спрайтов, настройки).
// ========================================================================

use mlua::{Lua, Table};

use crate::core::constants::*;
use crate::scripts::config::BalanceConfig;

#[test]
fn render_layers_are_strictly_ordered() {
    assert!(Z_MAP < Z_CARPET);
    assert!(Z_CARPET < Z_LIGHT);
    assert!(Z_LIGHT < Z_DECOR);
    assert!(Z_DECOR < Z_NPC);
    assert!(Z_NPC < Z_WEATHER);
    assert!(Z_WEATHER < Z_CURSOR);
    assert!(Z_CURSOR < Z_UI);
    assert!(Z_UI < Z_UI_TEXT);
    assert!(Z_UI_TEXT < Z_UI_TOOLTIP_BG);
    assert!(Z_UI_TOOLTIP_BG < Z_UI_TOOLTIP_TEXT);
    // Дверь рисуется на слое декора — под покупателями.
    assert_eq!(Z_DOOR, Z_DECOR);
}

#[test]
fn camera_and_zoom_bounds_are_sane() {
    assert!(ZOOM_MIN < ZOOM_MAX);
    assert!(TILE_HALF > 0.0);
    assert!(CAMERA_MAP_MIN_X < CAMERA_MAP_MAX_X);
    assert!(CAMERA_MAP_MIN_Y < CAMERA_MAP_MAX_Y);
    assert!(WORLD_OFFSET_X < 0.0);
    assert!(WORLD_OFFSET_Y > 0.0);
    assert!(SHADER_SCALE > 0.0);
}

#[test]
fn mode_tables_match_mode_count_and_textures_exist() {
    assert_eq!(CURSOR_TEX.len(), MODE_COUNT);
    assert_eq!(MODE_ICON_TEX.len(), MODE_COUNT);
    for path in CURSOR_TEX.iter().chain(MODE_ICON_TEX.iter()) {
        assert!(
            crate::core::asset::load_bytes(path).is_ok(),
            "нет текстуры режима: {path}"
        );
    }
    assert!(crate::core::asset::load_bytes(CURSOR_ERR_TEX).is_ok());
}

#[test]
fn inventory_tabs_have_textures_and_fit_the_grid() {
    assert_eq!(TAB_TEX.len(), 5);
    for path in TAB_TEX {
        assert!(crate::core::asset::load_bytes(path).is_ok(), "нет иконки таба: {path}");
    }
    let capacity = (INVENTORY_ROWS * INVENTORY_COLS) as usize;
    for list in [INV_REGULAR, INV_CARPETS, INV_WALLDECOR, INV_OUTDOOR, INV_LIGHT] {
        assert!(
            list.len() <= capacity,
            "вкладка не помещается в сетку: {} > {capacity}",
            list.len()
        );
    }
    // INV_NONE — это валидная нижняя левая ячейка сетки (ряд 4, колонка 0),
    // а не «пустая» координата. Тест фиксирует это, чтобы будущая проверка
    // `selected == INV_NONE` не сломала выбор первого предмета.
    assert_eq!(INV_NONE, (INVENTORY_ROWS - 1) * INVENTORY_COLS);
}

#[test]
fn all_objects_are_valid_and_assets_exist() {
    use crate::data::ALL_OBJECTS;

    for (i, obj) in ALL_OBJECTS.iter().enumerate() {
        assert!(obj.width > 0, "{}: width <= 0", obj.name);
        assert!(obj.height > 0, "{}: height <= 0", obj.name);
        assert!(obj.price > 0, "{}: price <= 0", obj.name);
        assert!(obj.texture_count[0] > 0 && obj.texture_count[1] > 0, "{}: пустой атлас", obj.name);
        assert!(
            obj.texture_frame[0] >= 0 && obj.texture_frame[0] < obj.texture_count[0],
            "{}: кадр X вне атласа",
            obj.name
        );
        assert!(
            obj.texture_frame[1] >= 0 && obj.texture_frame[1] < obj.texture_count[1],
            "{}: кадр Y вне атласа",
            obj.name
        );
        assert!(
            crate::core::asset::load_bytes(obj.path).is_ok(),
            "{}: нет текстуры {}",
            obj.name,
            obj.path
        );
        if obj.animated {
            assert!(
                obj.frame_paths.len() >= 2,
                "{}: animated, но кадров < 2",
                obj.name
            );
        }
        for frame in obj.frame_paths {
            assert!(
                crate::core::asset::load_bytes(frame).is_ok(),
                "{}: нет кадра анимации {frame}",
                obj.name
            );
        }
        for (_, other) in ALL_OBJECTS.iter().enumerate().skip(i + 1) {
            assert_ne!(obj.name, other.name, "дубликат имени объекта: {}", obj.name);
        }
    }
}

#[test]
fn inventory_lists_only_reference_catalog_objects() {
    use crate::data::ALL_OBJECTS;

    let known = |name: &str| ALL_OBJECTS.iter().any(|o| o.name == name);
    for name in INV_REGULAR
        .iter()
        .chain(INV_CARPETS.iter())
        .chain(INV_WALLDECOR.iter())
        .chain(INV_OUTDOOR.iter())
        .chain(INV_LIGHT.iter())
    {
        assert!(known(name), "предмета {name} нет в ALL_OBJECTS");
    }
    for name in SHOP_EXEMPT {
        assert!(known(name), "льготного предмета {name} нет в ALL_OBJECTS");
    }
}

#[test]
fn slot_icon_paths_for_inventory_tabs_exist() {
    for name in INV_REGULAR
        .iter()
        .chain(INV_CARPETS.iter())
        .chain(INV_WALLDECOR.iter())
        .chain(INV_OUTDOOR.iter())
        .chain(INV_LIGHT.iter())
    {
        let path = crate::core::util::slot_icon_path(name);
        assert!(
            crate::core::asset::load_bytes(&path).is_ok(),
            "{name}: нет иконки {path}"
        );
    }
}

#[test]
fn sprite_cache_key_changes_with_every_component() {
    use crate::core::util::sprite_cache_key;

    let base = sprite_cache_key("ui", "assets/tex/ui/shop.png", [0, 0], [1, 1], 1.0);
    assert_eq!(
        base,
        sprite_cache_key("ui", "assets/tex/ui/shop.png", [0, 0], [1, 1], 1.0)
    );
    assert_ne!(
        base,
        sprite_cache_key("map", "assets/tex/ui/shop.png", [0, 0], [1, 1], 1.0)
    );
    assert_ne!(
        base,
        sprite_cache_key("ui", "assets/tex/ui/shop_panel.png", [0, 0], [1, 1], 1.0)
    );
    assert_ne!(
        base,
        sprite_cache_key("ui", "assets/tex/ui/shop.png", [1, 0], [1, 1], 1.0)
    );
    assert_ne!(
        base,
        sprite_cache_key("ui", "assets/tex/ui/shop.png", [0, 0], [2, 2], 1.0)
    );
    assert_ne!(
        base,
        sprite_cache_key("ui", "assets/tex/ui/shop.png", [0, 0], [1, 1], 1.5)
    );
}

#[test]
fn day_night_factor_is_bounded_and_continuous() {
    use crate::scenes::game::day_night::DayNightCycle;

    let cfg = BalanceConfig::default();
    let at = |t: f64| {
        let mut c = DayNightCycle::new();
        c.tick(t);
        c.factor(&cfg)
    };

    assert_eq!(at(0.0), 0.0);
    assert_eq!(at(cfg.day_secs * 0.5), 0.0);
    assert_eq!(at((cfg.night_start_secs + cfg.night_secs) * 0.5), 1.0);
    assert_eq!(at(cfg.night_start_secs), 1.0);
    assert_eq!(at(cfg.night_secs), 1.0);

    // Закат/рассвет идут линейно через fade_secs.
    let mid_sunset = cfg.day_secs + cfg.fade_secs * 0.5;
    assert!((at(mid_sunset) - 0.5).abs() < 1e-5);
    let mid_dawn = cfg.night_secs + cfg.fade_secs * 0.5;
    assert!((at(mid_dawn) - 0.5).abs() < 1e-5);

    // На границах фаз нет скачков.
    let eps = 1e-6;
    for boundary in [cfg.day_secs, cfg.night_start_secs, cfg.night_secs, cfg.day_cycle_secs] {
        let before = at(boundary - eps);
        let after = at(boundary + eps);
        assert!(
            (before - after).abs() < 1e-4,
            "разрыв на границе {boundary}: {before} -> {after}"
        );
    }

    // Значение всегда в [0, 1].
    for step in 0..=240 {
        let v = at(step as f64 * cfg.day_cycle_secs / 240.0);
        assert!((0.0..=1.0).contains(&v), "factor вне диапазона: {v}");
    }
}

#[test]
fn day_night_time_string_wraps_every_cycle() {
    use crate::scenes::game::day_night::DayNightCycle;

    let cfg = BalanceConfig::default();
    let at = |t: f64| {
        let mut c = DayNightCycle::new();
        c.tick(t);
        c
    };

    assert_eq!(at(0.0).time_string(&cfg), "T:  00:00");
    assert_eq!(at(cfg.day_cycle_secs * 0.25).time_string(&cfg), "T:  06:00");
    assert_eq!(at(cfg.day_cycle_secs * 0.5).time_string(&cfg), "T:  12:00");
    assert_eq!(at(cfg.day_cycle_secs * 0.75).time_string(&cfg), "T:  18:00");
    assert_eq!(
        at(cfg.day_cycle_secs * 2.25).time_string(&cfg),
        at(cfg.day_cycle_secs * 0.25).time_string(&cfg)
    );

    let mut c = at(100.0);
    c.reset();
    assert_eq!(c.time_string(&cfg), "T:  00:00");
}

#[test]
fn balance_defaults_are_sane() {
    let cfg = BalanceConfig::default();
    assert!(cfg.day_secs < cfg.night_start_secs);
    assert!(cfg.night_start_secs < cfg.night_secs);
    assert!(cfg.night_secs < cfg.day_cycle_secs);
    assert!(cfg.fade_secs > 0.0);
    assert!(cfg.day_secs + cfg.fade_secs <= cfg.night_start_secs + 1e-9);
    assert!(cfg.night_secs + cfg.fade_secs <= cfg.day_cycle_secs + 1e-9);
    assert!(cfg.rent_interval_secs > 0.0);
    assert!(cfg.rent_amount > 0);
    assert!(cfg.start_money >= 0);
    assert!((0.0..=1.0).contains(&cfg.candy_chance));
    assert!(cfg.max_food_box > 0 && cfg.max_food_rack > 0 && cfg.max_food_candies > 0);
    assert!(cfg.candies_start_food <= cfg.max_food_candies);
    assert!(cfg.npc_speed > 0.0);
    assert!(cfg.npc_fade_speed > 0.0);
}

#[test]
fn balance_config_publishes_every_field_to_lua() {
    let mut cfg = BalanceConfig::default();
    cfg.shopper_spawn_interval = 1.5;
    cfg.object_prices.insert("box".to_string(), 7);
    cfg.font_path = "assets/test-font.otf".to_string();

    let lua = Lua::new();
    cfg.publish_to_lua(&lua).expect("publish_to_lua");
    let t: Table = lua.globals().get("CONFIG").expect("CONFIG");

    assert_eq!(t.get::<f64>("shopper_spawn_interval").unwrap(), cfg.shopper_spawn_interval);
    assert_eq!(t.get::<f64>("shopper_spawn_cooldown").unwrap(), cfg.shopper_spawn_cooldown);
    assert_eq!(t.get::<i64>("max_shoppers").unwrap(), cfg.max_shoppers as i64);
    assert_eq!(t.get::<f64>("food_regen_tick").unwrap(), cfg.food_regen_tick);
    assert_eq!(t.get::<i64>("food_regen_amount").unwrap(), cfg.food_regen_amount as i64);
    assert_eq!(t.get::<f64>("day_cycle_secs").unwrap(), cfg.day_cycle_secs);
    assert_eq!(t.get::<f64>("day_secs").unwrap(), cfg.day_secs);
    assert_eq!(t.get::<f64>("night_start_secs").unwrap(), cfg.night_start_secs);
    assert_eq!(t.get::<f64>("night_secs").unwrap(), cfg.night_secs);
    assert_eq!(t.get::<f64>("fade_secs").unwrap(), cfg.fade_secs);
    assert_eq!(t.get::<f64>("npc_speed").unwrap(), cfg.npc_speed as f64);
    assert_eq!(t.get::<f64>("walk_anim_interval").unwrap(), cfg.walk_anim_interval);
    assert_eq!(t.get::<f64>("npc_fade_speed").unwrap(), cfg.npc_fade_speed as f64);
    assert_eq!(t.get::<i64>("spawn_x").unwrap(), cfg.spawn_x as i64);
    assert_eq!(t.get::<i64>("spawn_y").unwrap(), cfg.spawn_y as i64);
    assert_eq!(t.get::<f64>("cassa_wait_secs").unwrap(), cfg.cassa_wait_secs);
    assert_eq!(t.get::<f64>("candy_wait_secs").unwrap(), cfg.candy_wait_secs);
    assert_eq!(t.get::<i64>("money_at_cassa").unwrap(), cfg.money_at_cassa as i64);
    assert_eq!(t.get::<i64>("money_at_candy").unwrap(), cfg.money_at_candy as i64);
    assert_eq!(t.get::<f64>("candy_chance").unwrap(), cfg.candy_chance);
    assert_eq!(t.get::<i64>("max_food_box").unwrap(), cfg.max_food_box as i64);
    assert_eq!(t.get::<i64>("max_food_rack").unwrap(), cfg.max_food_rack as i64);
    assert_eq!(t.get::<i64>("max_food_candies").unwrap(), cfg.max_food_candies as i64);
    assert_eq!(t.get::<i64>("candies_start_food").unwrap(), cfg.candies_start_food as i64);
    assert_eq!(t.get::<i64>("box_tex_threshold_1").unwrap(), cfg.box_tex_threshold_1 as i64);
    assert_eq!(t.get::<i64>("box_tex_threshold_2").unwrap(), cfg.box_tex_threshold_2 as i64);
    assert_eq!(t.get::<i64>("start_money").unwrap(), cfg.start_money as i64);
    assert_eq!(t.get::<i64>("rent_amount").unwrap(), cfg.rent_amount as i64);
    assert_eq!(t.get::<f64>("rent_interval_secs").unwrap(), cfg.rent_interval_secs);
    assert_eq!(t.get::<String>("font_path").unwrap(), cfg.font_path);

    let prices: Table = t.get("object_prices").unwrap();
    assert_eq!(prices.get::<i64>("box").unwrap(), 7);
}

#[test]
fn game_settings_serde_roundtrip_preserves_all_fields() {
    use crate::save::GameSettings;

    let s = GameSettings {
        vsync: false,
        zoom_speed: 0.15,
        music: false,
        sfx: true,
    };
    let json = serde_json::to_string(&s).unwrap();
    let back: GameSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.vsync, s.vsync);
    assert_eq!(back.zoom_speed, s.zoom_speed);
    assert_eq!(back.music, s.music);
    assert_eq!(back.sfx, s.sfx);

    let d = GameSettings::default();
    assert!(d.vsync);
    assert!((d.zoom_speed - 0.1).abs() < 1e-6);
    assert!(d.music);
    assert!(d.sfx);
}

#[test]
fn settings_apply_saved_clamps_zoom_speed() {
    use crate::save::GameSettings;

    let mut settings = crate::ui::settings::Settings::new();
    let (lo, hi) = (settings.zoom_speed.min, settings.zoom_speed.max);

    settings.apply_saved(&GameSettings {
        vsync: false,
        zoom_speed: hi + 10.0,
        music: false,
        sfx: false,
    });
    assert!(!settings.vsync.checked);
    assert!(!settings.music.checked);
    assert!(!settings.sfx.checked);
    assert_eq!(settings.zoom_speed.value, hi);

    settings.apply_saved(&GameSettings {
        vsync: true,
        zoom_speed: lo - 10.0,
        music: true,
        sfx: true,
    });
    assert!(settings.vsync.checked);
    assert_eq!(settings.zoom_speed.value, lo);
}

#[test]
fn inventory_tabs_and_grid_clicks_are_consistent() {
    use crate::ui::inventory::Inventory;

    let mut inv = Inventory::new();
    let expected: [&[&str]; 5] = [
        INV_REGULAR,
        INV_CARPETS,
        INV_WALLDECOR,
        INV_OUTDOOR,
        INV_LIGHT,
    ];
    for (tab, list) in expected.iter().enumerate() {
        inv.tab = tab as i32;
        assert_eq!(inv.items(), *list, "неверный список для вкладки {tab}");
    }
    // Неизвестный таб трактуется как вкладка света (текущее поведение).
    inv.tab = 99;
    assert_eq!(inv.items(), INV_LIGHT);

    inv.tab = 0;
    assert!(!inv.handle_grid_click(-1, 0));
    assert!(!inv.handle_grid_click(INVENTORY_COLS, 0));
    assert!(!inv.handle_grid_click(0, -1));
    assert!(!inv.handle_grid_click(0, INVENTORY_ROWS));
    assert!(inv.handle_grid_click(0, INVENTORY_ROWS - 1));
    assert_eq!(inv.selected, INV_NONE);
    assert_eq!(inv.selected_item_name(), Some(INV_REGULAR[0]));

    // Верхняя строка сетки — «хвост» списка, поэтому там пусто.
    assert!(inv.handle_grid_click(0, 0));
    assert_eq!(inv.selected, 0);
    assert_eq!(inv.selected_item_name(), None);
}

#[test]
fn ui_fit_scale_stays_in_range() {
    use crate::core::constants::{GAME_UI_HALF_H, GAME_UI_HALF_W, UI_MIN_SCALE};
    use crate::core::util::ui_fit_scale;

    let wide = ui_fit_scale(2.5, GAME_UI_HALF_W, GAME_UI_HALF_H);
    assert!(wide <= 1.0 && wide > 0.9, "на широком экране UI почти номинальный");
    let narrow = ui_fit_scale(0.2, GAME_UI_HALF_W, GAME_UI_HALF_H);
    assert!((UI_MIN_SCALE..=1.0).contains(&narrow));
    assert!((narrow - UI_MIN_SCALE).abs() < 1e-6);
    let mid = ui_fit_scale(1.0, GAME_UI_HALF_W, GAME_UI_HALF_H);
    assert!((UI_MIN_SCALE..=1.0).contains(&mid));
}
