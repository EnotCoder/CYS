// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  shader_pack_tests — discovery, WGSL loading and safe fallback.
// ========================================================================

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::core::shader_pack as shader;

static SHADER_LOCK: Mutex<()> = Mutex::new(());

const TEST_ID: &str = "__cys_test_shader";
const SOURCE: &str = "// test shader\n";

fn test_root() -> PathBuf {
    crate::core::asset::user_path(crate::core::shader_pack::shaders_root()).join(TEST_ID)
}

fn setup(files: &[(&str, &str)]) -> PathBuf {
    let root = test_root();
    let _ = fs::remove_dir_all(&root);
    for (name, body) in files {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    root
}

fn cleanup(root: &PathBuf) {
    crate::core::shader_pack::set_active(None);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bundled_shader_packs_are_well_formed() {
    for (id, name, marker) in [
        ("warm", "Comics", "COMIC_LEVELS"),
        ("retro", "Retro", "CRT_SCANLINE_STRENGTH"),
    ] {
        let manifest_path = format!("{}/{id}/shader.json", shader::shaders_root());
        let text = crate::core::asset::load_string(&manifest_path).unwrap();
        let manifest: shader::ShaderManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(manifest.name.as_deref(), Some(name));
        assert_eq!(manifest.file.as_deref(), Some("shader.wgsl"));

        let source_path = format!("{}/{id}/{}", shader::shaders_root(), manifest.file.unwrap());
        let source = crate::core::asset::load_string(&source_path).unwrap();
        assert!(source.contains("fn vs_main"), "{id} must define vs_main");
        assert!(source.contains("fn fs_main"), "{id} must define fs_main");
        assert!(source.contains(marker), "{id} must define {marker}");
    }
}

#[test]
fn shader_is_discovered_and_its_wgsl_is_loaded() {
    let _guard = SHADER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = setup(&[
        (
            "shader.json",
            r#"{"name":"Test Shader","author":"tester","version":"1"}"#,
        ),
        ("shader.wgsl", SOURCE),
    ]);

    shader::rescan();
    assert!(shader::list()
        .iter()
        .any(|s| s.id == TEST_ID && s.name == "Test Shader"));
    assert!(shader::active_id().is_none());
    assert!(shader::source().is_none());
    assert!(shader::error().is_none());

    shader::set_active(Some(TEST_ID));
    assert_eq!(shader::active_id().as_deref(), Some(TEST_ID));
    assert_eq!(shader::source().as_deref(), Some(SOURCE));
    assert!(shader::error().is_none());

    cleanup(&root);
}

#[test]
fn broken_manifest_falls_back_to_vanilla_and_reports_the_reason() {
    let _guard = SHADER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = setup(&[("shader.json", "{"), ("shader.wgsl", SOURCE)]);

    shader::set_active(Some(TEST_ID));
    assert!(shader::active_id().is_none());
    assert!(shader::source().is_none());
    let error = shader::error().expect("broken manifest must report an error");
    assert!(
        error.contains("invalid shader.json"),
        "unexpected error: {error}"
    );

    cleanup(&root);
}

#[test]
fn shader_file_cannot_escape_its_folder() {
    let _guard = SHADER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = setup(&[
        (
            "shader.json",
            r#"{"name":"Traversal","file":"../shaders.wgsl"}"#,
        ),
        ("shader.wgsl", SOURCE),
    ]);

    shader::set_active(Some(TEST_ID));
    assert!(shader::source().is_none());
    let error = shader::error().expect("unsafe path must report an error");
    assert!(
        error.contains("unsafe file path"),
        "unexpected error: {error}"
    );

    cleanup(&root);
}
