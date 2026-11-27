// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  jobs: фоновые потоки для чистых CPU-задач (декодирование текстур и
//  запись файла сохранения). Включены фичей `mt` только на десктопе.
//  Воркеры не трогают ECS, winit, Lua и GPU-ресурсы: они готовят
//  байты, а main-thread решает, когда загрузить их в текстуру/файл.
// ========================================================================

use crate::core::constants::{
    TEX_BOB_IDLE, TEX_BUTTON, TEX_MARK_IDLE, TEX_PLAYER_IDLE, TEX_SASHA_IDLE,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};

pub struct DecodedImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

struct DecodeJob {
    key: String,
    path: String,
}

struct SaveJob {
    path: String,
    json: String,
}

enum Done {
    Decoded {
        key: String,
        image: DecodedImage,
    },
    Saved {
        ok: bool,
    },
}

struct Hub {
    decode_tx: Option<Sender<DecodeJob>>,
    save_tx: Option<Sender<SaveJob>>,
    decoded_rx: Option<Receiver<Done>>,
    saved_rx: Option<Receiver<Done>>,
    _handles: Vec<JoinHandle<()>>,
}

static HUB: OnceLock<Mutex<Hub>> = OnceLock::new();
static DECODED: LazyLock<Mutex<HashMap<String, DecodedImage>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static PENDING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn enabled() -> bool {
    cfg!(all(feature = "mt", not(target_os = "android")))
}

fn hub() -> MutexGuard<'static, Hub> {
    HUB.get_or_init(|| {
        Mutex::new(if !enabled() {
            Hub {
                decode_tx: None,
                save_tx: None,
                decoded_rx: None,
                saved_rx: None,
                _handles: Vec::new(),
            }
        } else {
            let (decode_tx, decode_rx) = mpsc::channel::<DecodeJob>();
            let (save_tx, save_rx) = mpsc::channel::<SaveJob>();
            let (decoded_tx, decoded_rx) = mpsc::channel::<Done>();
            let (saved_tx, saved_rx) = mpsc::channel::<Done>();

            let decoded_worker = thread::Builder::new()
                .name("cys-decode".to_string())
                .spawn(move || {
                    while let Ok(job) = decode_rx.recv() {
                        let image = crate::core::asset::load_bytes(&job.path)
                            .ok()
                            .and_then(|bytes| {
                                crate::core::texture::Texture::decode_rgba(&bytes).ok()
                            })
                            .map(|(pixels, width, height)| DecodedImage {
                                pixels,
                                width,
                                height,
                            });
                        PENDING.lock().unwrap().remove(&job.key);
                        if let Some(image) = image {
                            let _ = decoded_tx.send(Done::Decoded { key: job.key, image });
                        }
                    }
                })
                .expect("failed to start texture worker");

            let saved_worker = thread::Builder::new()
                .name("cys-save".to_string())
                .spawn(move || {
                    while let Ok(job) = save_rx.recv() {
                        let ok =
                            crate::core::asset::save_data(&job.path, job.json.as_bytes()).is_ok();
                        let _ = saved_tx.send(Done::Saved { ok });
                    }
                })
                .expect("failed to start save worker");

            Hub {
                decode_tx: Some(decode_tx),
                save_tx: Some(save_tx),
                decoded_rx: Some(decoded_rx),
                saved_rx: Some(saved_rx),
                _handles: vec![decoded_worker, saved_worker],
            }
        })
    })
    .lock()
    .unwrap()
}

pub fn decode_key(path: &str) -> String {
    format!("{}|{path}", GENERATION.load(Ordering::Relaxed))
}

fn current_prefix() -> String {
    format!("{}|", GENERATION.load(Ordering::Relaxed))
}

/// Забирает готовый растр, если он относится к текущей версии ассетов.
pub fn take_decoded(key: &str) -> Option<DecodedImage> {
    if !enabled() || !key.starts_with(&current_prefix()) {
        return None;
    }
    DECODED.lock().unwrap().remove(key)
}

/// Сбрасывает фоновый кэш при смене пака/модов: старые результаты больше
/// не должны попадать в текстуры.
pub fn clear_decoded() {
    GENERATION.fetch_add(1, Ordering::Relaxed);
    DECODED.lock().unwrap().clear();
    PENDING.lock().unwrap().clear();
}

pub fn prewarm_textures(paths: &[String]) {
    if !enabled() {
        return;
    }
    let h = hub();
    let Some(tx) = h.decode_tx.as_ref() else {
        return;
    };
    for path in paths {
        let resolved = crate::core::texture_pack::resolve(path);
        let key = decode_key(&resolved);
        {
            let mut pending = PENDING.lock().unwrap();
            if pending.contains(&key) || DECODED.lock().unwrap().contains_key(&key) {
                continue;
            }
            pending.insert(key.clone());
        }
        if tx
            .send(DecodeJob {
                key: key.clone(),
                path: resolved,
            })
            .is_err()
        {
            PENDING.lock().unwrap().remove(&key);
            break;
        }
    }
}

/// Предзагружает текстуры, которые почти наверняка понадобятся в игре.
pub fn prewarm_game_textures() {
    if !enabled() {
        return;
    }
    let mut paths: Vec<String> = crate::data::catalog()
        .objects
        .iter()
        .map(|object| object.path.to_string())
        .collect();
    paths.extend(
        [
            "assets/tex/map/grass.png",
            "assets/tex/map/floor.png",
            "assets/tex/map/wall.png",
            TEX_BOB_IDLE,
            TEX_PLAYER_IDLE,
            TEX_SASHA_IDLE,
            TEX_MARK_IDLE,
            TEX_BUTTON,
        ]
        .iter()
        .map(|path| path.to_string()),
    );
    prewarm_textures(&paths);
}

/// Возвращает true, если запись ушла в поток; false — caller пишет синхронно.
pub fn save_async(path: &str, json: &str) -> bool {
    if !enabled() {
        return false;
    }
    let h = hub();
    let Some(tx) = h.save_tx.as_ref() else {
        return false;
    };
    tx.send(SaveJob {
        path: path.to_string(),
        json: json.to_string(),
    })
    .is_ok()
}

/// Должен вызываться один раз за кадр на главном потоке.
pub fn poll() {
    if !enabled() {
        return;
    }
    let h = hub();
    let prefix = current_prefix();
    if let Some(rx) = h.decoded_rx.as_ref() {
        while let Ok(Done::Decoded { key, image }) = rx.try_recv() {
            if key.starts_with(&prefix) {
                DECODED.lock().unwrap().insert(key, image);
            }
        }
    }
    if let Some(rx) = h.saved_rx.as_ref() {
        while let Ok(Done::Saved { ok }) = rx.try_recv() {
            if ok {
                crate::audio::play("save");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_results_from_old_generation_are_ignored() {
        let key = decode_key("assets/tex/map/grass.png");
        clear_decoded();
        assert!(take_decoded(&key).is_none());
    }

    #[test]
    fn disabled_workers_do_not_queue_anything() {
        if enabled() {
            return;
        }
        assert!(!save_async("unused.json", "{}"));
        assert!(take_decoded(&decode_key("unused.png")).is_none());
    }
}
