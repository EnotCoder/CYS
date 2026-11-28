// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Аудио-движок на rodio: загрузка файлов из sounds/, воспроизведение
// ========================================================================

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::cpal::{self, StreamError};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub const SOUND_DIR: &str = "assets/sounds";

const OUTPUT_BUFFER: Duration = Duration::from_millis(100);

fn output_buffer_size(sample_rate: u32) -> cpal::BufferSize {
    let frames = (OUTPUT_BUFFER.as_secs_f64() * f64::from(sample_rate)).round() as u32;
    cpal::BufferSize::Fixed(frames)
}

// Переключатели звука из настроек (data/settings.json): применяются глобально,
// чтобы музыка в меню и эффекты в игре уважали выбор пользователя.
static MUSIC_ENABLED: AtomicBool = AtomicBool::new(true);
static SFX_ENABLED: AtomicBool = AtomicBool::new(true);

fn should_log_stream_error(err: &StreamError) -> bool {
    !matches!(err, StreamError::BufferUnderrun)
}

fn on_stream_error(err: StreamError) {
    if should_log_stream_error(&err) {
        eprintln!("audio stream error: {err}");
    }
}

fn open_device_sink(device: cpal::Device) -> Option<MixerDeviceSink> {
    let sample_rate = device.default_output_config().ok()?.sample_rate();
    let builder = DeviceSinkBuilder::from_device(device).ok()?;
    let builder = builder
        .with_buffer_size(output_buffer_size(sample_rate))
        .with_error_callback(on_stream_error);
    builder.open_sink_or_fallback().ok()
}

fn open_output_sink() -> Option<MixerDeviceSink> {
    if let Some(device) = cpal::default_host().default_output_device() {
        if let Some(sink) = open_device_sink(device) {
            return Some(sink);
        }
    }

    let devices = cpal::default_host().output_devices().ok()?;
    for device in devices {
        let Ok(description) = device.description() else {
            continue;
        };
        if !description.driver().is_some_and(|driver| driver != "null") {
            continue;
        }
        if let Some(sink) = open_device_sink(device) {
            return Some(sink);
        }
    }
    None
}

// В rodio 0.22 VorbisDecoder::current_span_len() до первого сэмпла равен 0,
// поэтому repeat_infinite() (через buffered()) сразу считает поток пустым.
// Достаточно прочитать один сэмпл — после этого декодер знает длину блока.
fn looping_source(bytes: Vec<u8>) -> Option<impl Source<Item = f32> + Send + 'static> {
    let mut decoder = Decoder::new(Cursor::new(bytes)).ok()?;
    decoder.next()?;
    Some(decoder.repeat_infinite())
}

/// Включает/выключает фоновую музыку; при выключении останавливает текущий трек.
pub fn set_music_enabled(enabled: bool) {
    MUSIC_ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled {
        stop_music();
    }
}

/// Включает/выключает звуковые эффекты; при выключении останавливает
/// фоновые погодные лупы (дождь/снег).
pub fn set_sfx_enabled(enabled: bool) {
    SFX_ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled {
        stop_ambient();
    }
}

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
    // Активные фоновые лупы (погода), чтобы их можно было останавливать отдельно.
    ambient: Vec<Player>,
}

impl AudioEngine {
    pub fn new() -> Option<Self> {
        // Открываем устройство вывода; если его нет — движок не создаётся (None).
        let mut sink = open_output_sink()?;
        sink.log_on_drop(false);
        let mixer = sink.mixer().clone();
        let mut engine = Self {
            _sink: sink,
            mixer,
            clips: HashMap::new(),
            music: Vec::new(),
            ambient: Vec::new(),
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
        let Some(bytes) = self.clips.get(name) else {
            return;
        };
        // Байты клонируются, потому что rodio требует источник со временем жизни
        // 'static, а наши закэшированные данные живут внутри движка.
        let Ok(player) = rodio::play(&self.mixer, Cursor::new(bytes.clone())) else {
            return;
        };
        // detach() отвязывает плеер от локальной переменной: звук доживает сам.
        player.detach();
    }

    fn play_music_clip(&mut self, name: &str) -> bool {
        let Some(bytes) = self.clips.get(name) else {
            return false;
        };
        let Some(source) = looping_source(bytes.clone()) else {
            return false;
        };
        // Музыку зацикливаем и храним в списке, чтобы её можно было остановить.
        let player = Player::connect_new(&self.mixer);
        player.append(source);
        self.music.push(player);
        true
    }

    fn stop_music(&mut self) {
        // Останавливаем и убираем все музыкальные треки.
        for player in self.music.drain(..) {
            player.stop();
        }
    }

    // Зацикленный фоновый луп (погода): тише музыки, чтобы не перебивать её.
    fn play_ambient_clip(&mut self, name: &str) {
        let Some(bytes) = self.clips.get(name) else {
            return;
        };
        let Some(source) = looping_source(bytes.clone()) else {
            return;
        };
        let player = Player::connect_new(&self.mixer);
        player.append(source);
        player.set_volume(0.5);
        self.ambient.push(player);
    }

    fn stop_ambient(&mut self) {
        // Останавливаем все активные фоновые лупы.
        for player in self.ambient.drain(..) {
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
    if !SFX_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    with_engine(|engine| engine.play_clip(name));
}

/// Зациклить музыку по имени файла из sounds/ (без расширения).
/// Новый трек заменяет текущий, чтобы смена сезона не накладывала музыку.
/// Если сезонного трека нет, играем общий music.ogg.
pub fn play_music(name: &str) {
    if !MUSIC_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    with_engine(|engine| {
        engine.stop_music();
        if !engine.play_music_clip(name) && name != "music" {
            engine.play_music_clip("music");
        }
    });
}

// Остановить всю зацикленную музыку.
pub fn stop_music() {
    with_engine(|engine| engine.stop_music());
}

/// Зациклить фоновый звук (погода) по имени файла из sounds/ (без расширения).
pub fn play_ambient(name: &str) {
    if !SFX_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    with_engine(|engine| engine.play_ambient_clip(name));
}

// Остановить все фоновые лупы (погоду).
pub fn stop_ambient() {
    with_engine(|engine| engine.stop_ambient());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoverable_buffer_glitches_are_not_logged() {
        assert!(!should_log_stream_error(&StreamError::BufferUnderrun));
        assert!(should_log_stream_error(&StreamError::DeviceNotAvailable));
        assert!(should_log_stream_error(&StreamError::StreamInvalidated));
    }

    #[test]
    fn output_buffer_matches_100_ms_at_common_sample_rates() {
        assert_eq!(output_buffer_size(44_100), cpal::BufferSize::Fixed(4_410));
        assert_eq!(output_buffer_size(48_000), cpal::BufferSize::Fixed(4_800));
    }

    #[test]
    fn looping_vorbis_source_yields_samples() {
        for name in ["music", "rain", "snow"] {
            let path = format!("assets/sounds/{name}.ogg");
            let bytes = crate::core::asset::load_bytes(&path).unwrap();
            let samples: Vec<f32> = looping_source(bytes).unwrap().take(4096).collect();
            assert_eq!(samples.len(), 4096, "{path} produced no samples");
            assert!(samples.iter().any(|&sample| sample != 0.0), "{path} is silent");
        }
    }
}
