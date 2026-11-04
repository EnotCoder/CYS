// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  docs — контракт документации: пары RU/EN, карта исходников, одинаковые
//  блоки кода и живые относительные ссылки.
// ========================================================================

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("не читается {}: {e}", path.display()))
}

fn md_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("нет каталога {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(md_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn rel(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("нет {}: {e}", dir.display())).flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn rs_paths(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '/' | '.' | '-')))
        .filter(|t| t.starts_with("src/") && t.ends_with(".rs"))
        .map(str::to_string)
        .collect()
}

fn code_blocks(text: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(Vec::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push(line.to_string());
        }
    }
    blocks
}

fn links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = rest[..end].trim();
        rest = &rest[end..];
        if target.is_empty() || target.starts_with('#') {
            continue;
        }
        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            continue;
        }
        let path = target
            .split_whitespace()
            .next()
            .unwrap_or(target)
            .split('#')
            .next()
            .unwrap_or(target)
            .trim_matches(|c| c == '<' || c == '>');
        if !path.is_empty() {
            out.push(path.to_string());
        }
    }
    out
}

fn check_links(path: &Path) {
    let text = read(path);
    let parent = path.parent().unwrap();
    for link in links(&text) {
        let target = parent.join(&link);
        assert!(
            target.exists(),
            "{}: битая ссылка `{}` (нет {})",
            rel(path),
            link,
            target.display()
        );
    }
}

#[test]
fn docs_ru_and_en_are_paired() {
    let ru: BTreeSet<String> = md_files(&root().join("docs/ru"))
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let en: BTreeSet<String> = md_files(&root().join("docs/en"))
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    let missing_en: Vec<_> = ru.difference(&en).cloned().collect();
    let missing_ru: Vec<_> = en.difference(&ru).cloned().collect();
    assert!(
        missing_en.is_empty() && missing_ru.is_empty(),
        "документация не в парах: без en {missing_en:?}, без ru {missing_ru:?}"
    );
}

#[test]
fn source_map_covers_every_source_file() {
    let mut files = Vec::new();
    collect_rs(&root().join("src"), &mut files);
    let actual: BTreeSet<String> = files.iter().map(|p| rel(p)).collect();

    for lang in ["ru", "en"] {
        let path = root().join(format!("docs/{lang}/source-map.md"));
        let text = read(&path);
        let listed = rs_paths(&text);

        let missing: Vec<_> = actual.difference(&listed).cloned().collect();
        let extra: Vec<_> = listed.difference(&actual).cloned().collect();
        assert!(
            missing.is_empty(),
            "docs/{lang}/source-map.md: не описаны файлы {missing:?}"
        );
        assert!(
            extra.is_empty(),
            "docs/{lang}/source-map.md: описаны несуществующие файлы {extra:?}"
        );
    }
}

#[test]
fn code_blocks_match_between_languages() {
    for ru_path in md_files(&root().join("docs/ru")) {
        let name = ru_path.file_name().unwrap();
        let en_path = root().join("docs/en").join(name);
        let ru_blocks = code_blocks(&read(&ru_path));
        let en_blocks = code_blocks(&read(&en_path));
        assert_eq!(
            ru_blocks,
            en_blocks,
            "docs/ru/{} и docs/en/{} разошлись блоками кода",
            name.to_string_lossy(),
            name.to_string_lossy()
        );
    }
}

#[test]
fn all_relative_links_resolve() {
    for path in md_files(&root().join("docs")) {
        check_links(&path);
    }
    for name in ["README.md", "README.ru.md", "AGENTS.md"] {
        check_links(&root().join(name));
    }
}
