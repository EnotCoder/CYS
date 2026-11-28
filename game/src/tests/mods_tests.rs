// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  mods_tests — проверка манифестов, песочницы, событий и оверлеев модов.
// ========================================================================

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static MODS_LOCK: Mutex<()> = Mutex::new(());

const TEST_ID: &str = "__cys_test_mod";

fn test_root() -> PathBuf {
    crate::core::asset::user_path(crate::scripts::mods::mods_root()).join(TEST_ID)
}

fn write_mod(manifest: &str, lua: &str, texture: Option<&str>) {
    let root = test_root();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("mod.json"), manifest).unwrap();
    fs::write(root.join("main.lua"), lua).unwrap();
    if let Some(relative) = texture {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(crate::core::asset::user_path("assets/tex/dev_tools/black.png"), target).unwrap();
    }
}

fn cleanup() {
    let _ = fs::remove_dir_all(test_root());
    crate::scripts::mods::set_enabled(Vec::new());
}

#[test]
fn manifest_defaults_to_main_lua() {
    let manifest: crate::scripts::mods::ModManifest = serde_json::from_str(
        r#"{"name":"Test Mod","description":"desc","author":"me","version":"1.2.3"}"#,
    )
    .unwrap();
    assert_eq!(manifest.name.as_deref(), Some("Test Mod"));
    assert_eq!(manifest.description.as_deref(), Some("desc"));
    assert_eq!(manifest.author.as_deref(), Some("me"));
    assert_eq!(manifest.version.as_deref(), Some("1.2.3"));
    assert_eq!(manifest.entry, None);
    assert!(manifest.load_after.is_empty());
}

#[test]
fn bundled_mods_are_discovered() {
    let _guard = MODS_LOCK.lock().unwrap();
    let list = crate::scripts::mods::rescan();
    assert!(
        list.iter()
            .any(|m| m.id == "double_money" && m.name == "Double Money"),
        "double_money must be discoverable"
    );
    assert!(
        list.iter()
            .any(|m| m.id == "big_shop" && m.version == "1.0.0"),
        "big_shop must be discoverable"
    );
}

#[test]
fn mod_paths_stay_inside_the_mod_folder() {
    let base = crate::scripts::mods::mod_root("neon");
    let join = crate::core::texture_pack::safe_join;
    assert_eq!(
        join(&base, "main.lua").as_deref(),
        Some("assets/mods/neon/main.lua")
    );
    assert!(join(&base, "../config.lua").is_none());
    assert!(join(&base, "/etc/passwd").is_none());
    assert!(join(&base, "textures\\a.png").is_none());
}

#[test]
fn sandbox_blocks_unsafe_lua_globals() {
    let _guard = MODS_LOCK.lock().unwrap();
    write_mod(
        r#"{"name":"Sandbox Test"}"#,
        "local f = io.open('secret.txt')",
        None,
    );
    crate::scripts::mods::set_enabled(vec![TEST_ID.to_string()]);
    assert!(
        crate::scripts::mods::errors().contains_key(TEST_ID),
        "unsafe Lua must fail and be reported"
    );
    assert!(!crate::scripts::mods::loaded_ids().contains(&TEST_ID.to_string()));
    cleanup();
}

#[test]
fn event_handlers_can_mark_events_handled() {
    let _guard = MODS_LOCK.lock().unwrap();
    write_mod(
        r#"{"name":"Event Test"}"#,
        &format!(
            "mod.on('{}', function(data, dt) return true end)",
            crate::scripts::mods::EVENT_MONEY_CHANGED
        ),
        None,
    );
    crate::scripts::mods::set_enabled(vec![TEST_ID.to_string()]);
    assert!(crate::scripts::mods::emit(
        crate::scripts::mods::EVENT_MONEY_CHANGED,
        &[("delta", crate::scripts::mods::ModArg::Int(5))],
        0.0,
    ));
    cleanup();
}

#[test]
fn bundled_mods_override_balance_and_item_catalog() {
    let _guard = MODS_LOCK.lock().unwrap();
    crate::scripts::mods::set_enabled(vec!["double_money".to_string(), "big_shop".to_string()]);
    let cfg = crate::scripts::config::BalanceConfig::load();
    assert_eq!(cfg.money_at_cassa, 10);
    assert_eq!(cfg.money_at_candy, 2);
    assert_eq!(cfg.start_money, 300);

    let items = crate::scripts::items::load().expect("items.lua must load");
    assert!(
        items
            .iter()
            .any(|item| item.name == "neon_box" && item.price == 80),
        "registered mod item must appear in the catalog"
    );

    crate::scripts::mods::set_enabled(Vec::new());
    let cfg = crate::scripts::config::BalanceConfig::load();
    assert_eq!(cfg.money_at_cassa, 5);
    assert_eq!(cfg.start_money, 150);
}

#[test]
fn mod_texture_layer_applies_without_active_pack() {
    let _guard = MODS_LOCK.lock().unwrap();
    crate::core::init();
    write_mod(
        r#"{"name":"Texture Test"}"#,
        "",
        Some("textures/map/grass.png"),
    );
    crate::scripts::mods::set_enabled(vec![TEST_ID.to_string()]);
    assert!(crate::scripts::mods::texture_layers().contains(&TEST_ID.to_string()));
    let resolved = crate::core::texture_pack::resolve("assets/tex/map/grass.png");
    assert!(
        resolved.contains(TEST_ID),
        "mod texture layer must win even with no active pack: {resolved}"
    );
    cleanup();
    let resolved = crate::core::texture_pack::resolve("assets/tex/map/grass.png");
    assert!(
        !resolved.contains(TEST_ID),
        "disabled mod must not affect vanilla textures"
    );
}

#[test]
fn manifest_use_flag_is_optional() {
    let on: crate::scripts::mods::ModManifest =
        serde_json::from_str(r#"{"name":"On","use":true}"#).unwrap();
    let off: crate::scripts::mods::ModManifest =
        serde_json::from_str(r#"{"name":"Off","use":false}"#).unwrap();
    let missing: crate::scripts::mods::ModManifest =
        serde_json::from_str(r#"{"name":"Missing"}"#).unwrap();
    assert_eq!(on.use_mod, Some(true));
    assert_eq!(off.use_mod, Some(false));
    assert_eq!(missing.use_mod, None);
}

#[test]
fn enabled_from_honors_use_flag_and_keeps_saved_order() {
    let info = |id: &str, use_mod: Option<bool>| crate::scripts::mods::ModInfo {
        id: id.to_string(),
        name: id.to_string(),
        description: String::new(),
        author: String::new(),
        version: String::new(),
        entry: String::new(),
        load_after: Vec::new(),
        game_version: String::new(),
        use_mod,
    };
    let available = vec![
        info("a", Some(true)),
        info("b", Some(false)),
        info("c", None),
        info("d", Some(true)),
    ];
    let saved = vec![
        "c".to_string(),
        "b".to_string(),
        "a".to_string(),
        "gone".to_string(),
    ];
    assert_eq!(
        crate::scripts::mods::enabled_from(&available, &saved),
        vec!["c", "a", "d"],
        "`use:false` must win over settings, `use:true` mods are appended, \
         unknown ids are dropped"
    );
}

#[test]
fn write_use_flag_updates_manifest_and_state() {
    let _guard = MODS_LOCK.lock().unwrap();
    write_mod(r#"{"name":"Toggle Test","use":false}"#, "", None);
    crate::scripts::mods::rescan();
    assert_eq!(
        crate::scripts::mods::list()
            .into_iter()
            .find(|m| m.id == TEST_ID)
            .unwrap()
            .use_mod,
        Some(false)
    );
    crate::scripts::mods::write_use_flag(TEST_ID, true).unwrap();
    let raw = fs::read_to_string(test_root().join("mod.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["use"], serde_json::Value::Bool(true));
    assert_eq!(
        crate::scripts::mods::list()
            .into_iter()
            .find(|m| m.id == TEST_ID)
            .unwrap()
            .use_mod,
        Some(true),
        "in-memory state must follow the written flag"
    );
    cleanup();
}

#[test]
fn bundled_manifests_declare_use_flag() {
    for id in ["double_money", "big_shop"] {
        let shipped = crate::core::asset::user_path("assets/mods").join(id).join("mod.json");
        let raw = fs::read_to_string(&shipped).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            parsed["use"],
            serde_json::Value::Bool(true),
            "{} must ship with `use: true`",
            shipped.display()
        );
    }
}
