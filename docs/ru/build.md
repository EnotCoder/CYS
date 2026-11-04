# Сборка

[English](../en/build.md) · Русский

## Требования

- Rust с поддержкой edition 2021 (1.82+ — безопасный минимум);
- GPU с Vulkan (Linux/Android) или Dx12 (Windows);
- для Android: Android SDK, NDK, `cargo-apk` и тулчейны целей.

`Cargo.toml` фиксирует `wgpu = "30"`, `winit = "0.30"`, `specs = "0.18"`,
`mlua = "0.10"` (vendored Lua 5.4) и `rodio = "0.22"`.

## Десктоп

```sh
cargo build --release
cargo run --release
```

Debug-сборка тоже работает и хранит символы, но release-профиль настроен под
поставку:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

`lto` и `codegen-units = 1` меняют время компиляции на время кадра; `strip`
уменьшает бинарник; `panic = "abort"` убирает разворачивание стека. Отдельного
`dev`-тюнинга нет.

## Windows

Обычно крейт собирается на Windows с бэкендом Dx12 по умолчанию. Для
кросс-сборки с Linux через mingw `Cargo.toml` уже отключает Vulkan и оставляет
только Dx12 + WGSL:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
wgpu = { version = "30", default-features = false, features = ["dx12", "wgsl"] }
```

`build/main.rs` встраивает `build/app.ico` в `.exe` через `winres`.
Build-скрипт компилируется под хост, поэтому цель он определяет по
`CARGO_CFG_TARGET_OS`, а не через `cfg(target_os)` — иначе при кросс-сборке
иконка молча пропадает.

## Android

В манифесте лежат метаданные `cargo-apk`:

| Настройка | Значение |
|---|---|
| package | `ru.enotcoder.createshop` |
| цели | `aarch64-linux-android`, `x86_64-linux-android` |
| ассеты | `assets/` упаковываются в APK |
| ресурсы | `res/` |
| min SDK / target SDK | 26 / 34 |
| ориентация | landscape |
| тема | `Theme.NoTitleBar.Fullscreen` |
| label | `Create your shop (CYS)` |

Сборка:

```sh
cargo apk build --release
```

Подпись — `release.keystore` (alias `cys-alias`). Keystore указан в манифесте;
не коммить настоящие release-ключи в публичный репозиторий — замени
плейсхолдеры перед публикацией сборки.

На Android точка входа — `android_main` в `src/main/android.rs`: он зовёт
`cys::run()` и настраивает JNI `EditText` для ввода имени мира. Системная
кнопка Back работает как `Esc`: закрыть панель или открыть настройки/паузу.

## Конфигурация в рантайме

- `assets/scripts/config.lua` — баланс, путь к шрифту, точка спавна;
- `settings.json` — VSync, зум, музыка и звуки, создаётся при первом изменении;
- `worlds/` — сохранения игрока, создаются по требованию.

Путь к шрифту должен указывать на файл, читаемый через `core::asset`; смена
пути требует перезапуска, потому что `TextRenderer` кэширует шрифт на старте.

## VSync

Панель настроек переключает present mode поверхности на лету:

- VSync включён → `PresentMode::Fifo`;
- VSync выключен → `PresentMode::Mailbox`, если адаптер его поддерживает,
  иначе первый поддерживаемый режим.

Перезапуск не нужен: сцена возвращает `SceneAction::VsyncToggle(bool)`, а
`App` перенастраивает surface.

## Диагностика

| Симптом | Вероятная причина |
|---|---|
| чёрное окно без паники | ошибка валидации шейдера/пайплайна; запусти с `RUST_LOG=wgpu_core=warn` |
| `Failed to find ColorBuffer` на эмуляторе Android | GPU-контекст пересоздавался при resume; CYS переиспользует его и пересоздаёт только surface |
| вторая лампа не светит | размер `LightData` не совпал с `std430`-шагом WGSL (64 байта) |
| текстуры мыльные | сэмплер создан с `Linear`; пиксель-арту нужен `Nearest` |
| нет шрифта | `font_path` в `config.lua` указывает вне `assets/` или файла нет |
