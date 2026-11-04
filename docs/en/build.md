# Build

[Русский](../ru/build.md) · English

## Requirements

- Rust with edition 2021 support (1.82 or newer is a safe baseline);
- a GPU with Vulkan (Linux/Android) or Dx12 (Windows);
- for Android: Android SDK, NDK, `cargo-apk`, and the target toolchains.

`Cargo.toml` pins `wgpu = "30"`, `winit = "0.30"`, `specs = "0.18"`,
`mlua = "0.10"` (vendored Lua 5.4) and `rodio = "0.22"`.

## Desktop

```sh
cargo build --release
cargo run --release
```

Debug builds work too and keep debug symbols, but the release profile is
tuned for shipping:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

`lto` and `codegen-units = 1` trade compile time for frame time; `strip` keeps
the binary small; `panic = "abort"` removes unwinding. There is no separate
`dev` tuning.

## Windows

The crate is normally built on Windows with the default Dx12 backend. For
cross-compiling from Linux with mingw, `Cargo.toml` already disables Vulkan and
keeps only Dx12 + WGSL:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
wgpu = { version = "30", default-features = false, features = ["dx12", "wgsl"] }
```

`build/main.rs` embeds `build/app.ico` into the `.exe` via `winres`. The build
script is compiled for the host, so it detects the target with
`CARGO_CFG_TARGET_OS` instead of `cfg(target_os)` — otherwise the icon silently
disappears during cross-compilation.

## Android

The manifest contains the `cargo-apk` metadata:

| Setting | Value |
|---|---|
| package | `ru.enotcoder.createshop` |
| targets | `aarch64-linux-android`, `x86_64-linux-android` |
| assets | `assets/` packed into the APK |
| resources | `res/` |
| min SDK / target SDK | 26 / 34 |
| orientation | landscape |
| theme | `Theme.NoTitleBar.Fullscreen` |
| label | `Create your shop (CYS)` |

Build:

```sh
cargo apk build --release
```

Signing uses `release.keystore` (alias `cys-alias`). The keystore is listed in
the manifest; do not commit real release keys to a public repository — replace
the placeholder values before publishing a build.

On Android the entry point is `android_main` in `src/main/android.rs`, which
calls `cys::run()` and sets up the JNI `EditText` used for world names. The
system Back button is treated as `Esc`: close a panel, or open settings/pause.

## Runtime configuration

- `assets/scripts/config.lua` — balance, font path, spawn point;
- `settings.json` — VSync, zoom, music and SFX, created on first change;
- `worlds/` — user saves, created on demand.

The font path must point at a file readable through `core::asset`; changing it
requires a restart because `TextRenderer` caches the font at startup.

## VSync

The settings panel switches the surface present mode at runtime:

- VSync on → `PresentMode::Fifo`;
- VSync off → `PresentMode::Mailbox` when the adapter supports it, otherwise
  the first supported mode.

No restart is needed; the scene returns `SceneAction::VsyncToggle(bool)` and
`App` reconfigures the surface.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| black window, no panic | shader/pipeline validation error; run with `RUST_LOG=wgpu_core=warn` |
| `Failed to find ColorBuffer` on Android emulator | GPU context was recreated on resume; CYS reuses it and only recreates the surface |
| second light does not work | `LightData` size does not match the WGSL `std430` stride (64 bytes) |
| textures look blurry | a sampler was created with `Linear`; pixel art must use `Nearest` |
| font missing | `font_path` in `config.lua` points outside `assets/` or the file is absent |
