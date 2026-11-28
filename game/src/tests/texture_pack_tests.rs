// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  texture_pack_tests — проверка оверлея файлов и переопределений pack.json.
// ========================================================================

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static PACK_LOCK: Mutex<()> = Mutex::new(());

const TEST_ID: &str = "__cys_test_pack";

fn test_root() -> PathBuf {
    crate::core::asset::user_path(crate::core::texture_pack::packs_root()).join(TEST_ID)
}

fn copy_png(relative: &str) {
    let root = test_root();
    let target = root.join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(crate::core::asset::user_path("assets/tex/dev_tools/black.png"), target).unwrap();
}

#[test]
fn pack_overlay_overrides_and_vanilla_fallback_work() {
    let _guard = PACK_LOCK.lock().unwrap();
    let root = test_root();
    let _ = fs::remove_dir_all(&root);

    for file in [
        "textures/map/grass.png",
        "textures/alt_box.png",
        "textures/box_alt.png",
        "textures/arcade_base.png",
        "textures/arcade_0.png",
        "textures/arcade_1.png",
        "textures/bob_idle.png",
        "textures/bob_walk.png",
    ] {
        copy_png(file);
    }
    fs::copy(
        crate::core::asset::user_path("assets/tex/map/grass.png"),
        root.join("textures/map/grass.png"),
    )
    .unwrap();
    fs::write(
        root.join("pack.json"),
        r#"{
        "name": "Test Pack",
        "textures": {
            "assets/tex/map/floor.png": "textures/map/grass.png",
            "assets/tex/ui/button/main.png": "../../outside.png"
        },
        "objects": {
            "box": {
                "texture": "textures/box_alt.png",
                "frame": [1, 1],
                "atlas": [2, 2],
                "animated": false,
                "fps": 4.0,
                "loop": false
            },
            "arcade_machine": {
                "texture": "textures/arcade_base.png",
                "frames": ["textures/arcade_0.png", "textures/arcade_1.png"],
                "fps": 6.0
            }
        },
        "npcs": {
            "bob": {
                "idle": ["textures/bob_idle.png"],
                "walk": ["textures/bob_walk.png"],
                "fps": 6.0
            }
        }
    }"#,
    )
    .unwrap();

    crate::core::texture_pack::set_active(Some(TEST_ID));
    assert_eq!(
        crate::core::texture_pack::active_id().as_deref(),
        Some(TEST_ID)
    );
    assert!(crate::core::texture_pack::list()
        .iter()
        .any(|p| p.id == TEST_ID && p.name == "Test Pack"));

    use crate::core::texture_pack as pack;
    let mirror = format!("assets/texture_pack/{TEST_ID}/textures/map/grass.png");
    assert_eq!(pack::resolve("assets/tex/map/grass.png"), mirror);
    // Суффикс @WxH — не часть имени файла.
    assert_eq!(pack::resolve("assets/tex/map/grass.png@2x1"), mirror);
    // Явная карта работает и для путей вне зеркала textures/.
    assert_eq!(pack::resolve("assets/tex/map/floor.png"), mirror);
    // Путь с .. из pack.json отвергается.
    assert_eq!(
        pack::resolve("assets/tex/ui/button/main.png"),
        "assets/tex/ui/button/main.png",
    );

    assert_eq!(
        pack::object_texture("box", "vanilla"),
        format!("assets/texture_pack/{TEST_ID}/textures/box_alt.png")
    );
    assert_eq!(pack::object_frame("box", [0, 0]), [1, 1]);
    assert_eq!(pack::object_atlas("box", [1, 1]), [2, 2]);
    assert!(!pack::object_animated("box", true));
    assert_eq!(pack::object_fps("box"), 4.0);
    assert!(!pack::object_loop("box"));
    assert_eq!(
        pack::object_frames("arcade_machine", &["vanilla_0", "vanilla_1"]),
        vec![
            format!("assets/texture_pack/{TEST_ID}/textures/arcade_0.png"),
            format!("assets/texture_pack/{TEST_ID}/textures/arcade_1.png"),
        ],
    );
    let arcade = format!("assets/texture_pack/{TEST_ID}/textures/arcade_base.png");
    assert_eq!(pack::object_texture("arcade_machine", "vanilla"), arcade);
    let (idle, walk, fps) = pack::npc_textures("bob", "idle", "w1", "w2");
    assert_eq!(
        idle,
        vec![format!("assets/texture_pack/{TEST_ID}/textures/bob_idle.png")]
    );
    assert_eq!(
        walk,
        vec![format!("assets/texture_pack/{TEST_ID}/textures/bob_walk.png")]
    );
    assert_eq!(fps, Some(6.0));

    let mut ecs = crate::tests::helpers::ecs_with_map();
    let cursor = ecs.add_cursor(0.0, 0.0, crate::core::constants::CURSOR_TEX[1]);
    let slots = [crate::data::make_slot("arcade_machine")];
    crate::input::cursor::update_cursor_preview(
        &mut ecs,
        crate::core::constants::MODE_BUILD,
        &slots,
        0,
        cursor,
    );
    use specs::WorldExt;
    let sprites = ecs.world.read_storage::<crate::SpriteComponent>();
    let preview_paths: Vec<String> = ecs
        .cursor_preview
        .iter()
        .filter_map(|entity| sprites.get(*entity).map(|s| s.texture_path.to_string()))
        .collect();
    assert!(preview_paths.iter().any(|path| path.starts_with(&arcade)));

    crate::core::texture_pack::set_active(None);
    assert_eq!(
        pack::resolve("assets/tex/map/grass.png"),
        "assets/tex/map/grass.png"
    );
    assert_eq!(pack::object_texture("box", "vanilla"), "vanilla");
    assert_eq!(pack::object_fps("box"), 1.0);
    assert!(pack::object_loop("box"));

    fs::remove_dir_all(&root).unwrap();
}
