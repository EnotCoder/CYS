// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Аудио-движок на rodio: загрузка файлов из sounds/, воспроизведение
// ========================================================================

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub const SOUND_DIR: &str = "sounds";

pub struct AudioEngine {
    // _sink держится живым на всё время жизни движка: именно он является
    // владельцем устройства вывода, и пока он жив — звук продолжает играть.
    _sink: MixerDeviceSink,
    // Микшер — точка входа для всех проигрываемых источников звука.
    mixer: rodio::mixer::Mixer,
    // Закэшированные звуки из sounds/ по имени файла без расширения.
    clips: HashMap<String, Vec<u8>>,
    // Активные музыкальные треки (зацикленные), чтобы их можно было остановить.
    music: Vec<Player>,
}

impl AudioEngine {
    pub fn new() -> Option<Self> {
        // Открываем устройство вывода; если его нет — движок не создаётся (None).
        let mut sink = DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        let mixer = sink.mixer().clone();
        let mut engine = Self {
            _sink: sink,
            mixer,
            clips: HashMap::new(),
            music: Vec::new(),
        };
        engine.load_all(SOUND_DIR);
        Some(engine)
    }

    fn load_all(&mut self, dir: &str) {
        // Грузим все файлы из каталога звуков.
        for name in crate::core::asset::list_dir(dir) {
            let path = format!("{}/{}", dir, name);
            self.load(&path);
        }
    }

    fn load(&mut self, path: &str) {
        // Имя клипа берём из имени файла без расширения.
        let Some(name) = Path::new(path).file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        if name.is_empty() {
            return;
        }
        if let Ok(bytes) = crate::core::asset::load_bytes(path) {
            self.clips.insert(name.to_string(), bytes);
        }
    }

    fn play_clip(&self, name: &str) {
        let Some(bytes) = self.clips.get(name) else { return };
        // Байты клонируются, потому что rodio требует источник со временем жизни
        // 'static, а наши закэшированные данные живут внутри движка.
        let Ok(player) = rodio::play(&self.mixer, Cursor::new(bytes.clone())) else {
            return;
        };
        // detach() отвязывает плеер от локальной переменной: звук доживает сам.
        player.detach();
    }

    fn play_music_clip(&mut self, name: &str) {
        let Some(bytes) = self.clips.get(name) else { return };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes.clone())) else {
            return;
        };
        // Музыку зацикливаем и храним в списке, чтобы её можно было остановить.
        let player = Player::connect_new(&self.mixer);
        player.append(decoder.repeat_infinite());
        self.music.push(player);
    }

    fn stop_music(&mut self) {
        // Останавливаем и убираем все музыкальные треки.
        for player in self.music.drain(..) {
            player.stop();
        }
    }
}

// ========================================================================
//  Глобальный доступ к звуку из любой подсистемы
// ========================================================================

// Синглтон-хранилище: инициализируется лениво один раз (OnceLock), а Mutex
// защищает доступ из нескольких потоков; Option означает «движка нет»,
// если устройство вывода недоступно (например, нет аудиокарты).
static ENGINE: OnceLock<Mutex<Option<AudioEngine>>> = OnceLock::new();

// Единая точка входа для всех публичных функций: блокирует движок
// и выполняет переданное замыкание, если движок удалось создать.
fn with_engine(f: impl FnOnce(&mut AudioEngine)) {
    let engine = ENGINE.get_or_init(|| Mutex::new(AudioEngine::new()));
    if let Ok(mut guard) = engine.lock() {
        if let Some(engine) = guard.as_mut() {
            f(engine);
        }
    }
}

// Явная инициализация (например, для предзагрузки звуков при старте).
pub fn init() {
    let _ = ENGINE.get_or_init(|| Mutex::new(AudioEngine::new()));
}

/// Воспроизвести звук один раз по имени файла из sounds/ (без расширения).
pub fn play(name: &str) {
    with_engine(|engine| engine.play_clip(name));
}

/// Зациклить музыку по имени файла из sounds/ (без расширения).
pub fn play_music(name: &str) {
    with_engine(|engine| engine.play_music_clip(name));
}

// Остановить всю зацикленную музыку.
pub fn stop_music() {
    with_engine(|engine| engine.stop_music());
}