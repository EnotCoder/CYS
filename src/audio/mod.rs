// ========================================================================
//  Аудио-движок на rodio: загрузка файлов из sounds/, воспроизведение
// ========================================================================

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub const SOUND_DIR: &str = "sounds";

pub struct AudioEngine {
    _sink: MixerDeviceSink,
    mixer: rodio::mixer::Mixer,
    clips: HashMap<String, Vec<u8>>,
    music: Vec<Player>,
}

impl AudioEngine {
    pub fn new() -> Option<Self> {
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
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            self.load(&entry.path());
        }
    }

    fn load(&mut self, path: &Path) {
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        if name.is_empty() {
            return;
        }
        if let Ok(bytes) = fs::read(path) {
            self.clips.insert(name.to_string(), bytes);
        }
    }

    fn play_clip(&self, name: &str) {
        let Some(bytes) = self.clips.get(name) else { return };
        let Ok(player) = rodio::play(&self.mixer, Cursor::new(bytes.clone())) else {
            return;
        };
        player.detach();
    }

    fn play_music_clip(&mut self, name: &str) {
        let Some(bytes) = self.clips.get(name) else { return };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes.clone())) else {
            return;
        };
        let player = Player::connect_new(&self.mixer);
        player.append(decoder.repeat_infinite());
        self.music.push(player);
    }

    fn stop_music(&mut self) {
        for player in self.music.drain(..) {
            player.stop();
        }
    }
}

// ========================================================================
//  Глобальный доступ к звуку из любой подсистемы
// ========================================================================

static ENGINE: OnceLock<Mutex<Option<AudioEngine>>> = OnceLock::new();

fn with_engine(f: impl FnOnce(&mut AudioEngine)) {
    let engine = ENGINE.get_or_init(|| Mutex::new(AudioEngine::new()));
    if let Ok(mut guard) = engine.lock() {
        if let Some(engine) = guard.as_mut() {
            f(engine);
        }
    }
}

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

pub fn stop_music() {
    with_engine(|engine| engine.stop_music());
}