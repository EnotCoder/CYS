# Create your Shop (CYS)

[English](README.md) · [Русский](README.ru.md)

> **Beta** — the game is in active beta. Features may change and bugs are
> expected.

[![license](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.82%2B-blue)
![wgpu](https://img.shields.io/badge/wgpu-30-orange)
![platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20Android-lightgrey)

A 2D shop simulator written in Rust on top of `wgpu`. Build your own shop:
lay floors, walls and carpets, place racks, registers and decor, stock the
shelves, serve shopper NPCs and keep the business afloat.

![Gameplay](img/prev.png)

## Build & run

```sh
cargo build --release
cargo run --release
```

Requires Rust edition 2021 and a GPU with Vulkan (Linux/Android) or Dx12
(Windows). For Android see [docs/en/build.md](docs/en/build.md).

## Controls

| Action | Input |
|---|---|
| Place / remove / interact | `LMB` or `F` |
| Cycle mode (move/interact/build/delete) | `Tab` or the mode icon |
| Open / close inventory | `E` |
| Select hotbar slot | Click the slot |
| Toggle shop open/closed | Click the active icon |
| Zoom | `Scroll`, `K` / `L`, pinch |
| Move camera (mode `move`) | Drag mouse/finger, arrow keys |
| Settings (pauses the game) | `Esc` / Android Back button |
| Quick save | `Ctrl+S` (load via the world menu) |
| Enter basement | Click the basement stairs |
| Back to menu | `R` (when bankrupt) |

On Android: tap = click, one-finger drag moves the camera only in `move` mode,
two-finger pinch = zoom, the system Back button closes panels or opens the
pause menu. `move` is the default mode; switch to interact/build/delete to work
with the world.

## Features

- **Build mode** — floor, walls, carpets, outdoor decor, flowers.
- **Objects** — racks, boxes, registers, a candy display, arcade machines,
  fences, a basement staircase and a shop.
- **Shoppers** — A* pathfinding, queueing for a register, buying food and
  candy, walking in through an animated door.
- **Economy** — prices, rent, bankruptcy, shop open/closed state, purchasable
  object unlocks.
- **World** — day/night cycle with dynamic 2D point lighting, four seasons with
  snow and rain particles.
- **Basement** — a second level with two-way transitions and its own map.
- **Worlds** — create, name and delete worlds in a Minecraft-style menu;
  autosave on exit, quick save on `Ctrl+S`.
- **Pause** — settings freeze rent, day/night, shoppers and weather.
- **Settings** — VSync, zoom speed, music and SFX, persisted in `settings.json`.
- **Balance in Lua** — `assets/scripts/config.lua` and `assets/scripts/npc.lua`
  drive timings, prices and shopper decisions.

## Documentation

The full developer documentation lives in [`docs/`](docs/README.md). Russian is
the source language, English mirrors it page by page.

| Topic | English | Русский |
|---|---|---|
| Start here | [docs/en/architecture.md](docs/en/architecture.md) | [docs/ru/architecture.md](docs/ru/architecture.md) |
| Source map | [docs/en/source-map.md](docs/en/source-map.md) | [docs/ru/source-map.md](docs/ru/source-map.md) |
| Rendering | [docs/en/rendering.md](docs/en/rendering.md) | [docs/ru/rendering.md](docs/ru/rendering.md) |
| ECS | [docs/en/ecs.md](docs/en/ecs.md) | [docs/ru/ecs.md](docs/ru/ecs.md) |
| Gameplay | [docs/en/gameplay.md](docs/en/gameplay.md) | [docs/ru/gameplay.md](docs/ru/gameplay.md) |
| Saves & maps | [docs/en/world-format.md](docs/en/world-format.md) | [docs/ru/world-format.md](docs/ru/world-format.md) |
| Lua scripts | [docs/en/scripts.md](docs/en/scripts.md) | [docs/ru/scripts.md](docs/ru/scripts.md) |
| Assets | [docs/en/assets.md](docs/en/assets.md) | [docs/ru/assets.md](docs/ru/assets.md) |
| Build | [docs/en/build.md](docs/en/build.md) | [docs/ru/build.md](docs/ru/build.md) |
| Tests | [docs/en/testing.md](docs/en/testing.md) | [docs/ru/testing.md](docs/ru/testing.md) |
| Contributing | [docs/en/contributing.md](docs/en/contributing.md) | [docs/ru/contributing.md](docs/ru/contributing.md) |

The docs are checked by `cargo test`: RU/EN pages must pair up, every `.rs`
file must appear in the source map, code blocks must match between languages,
and all relative links must resolve.

## Architecture

```text
src/
├── main/     — entry point, App, winit event loop, Android entry
├── core/     — wgpu kernel: pipeline, textures, render, shaders, lighting
├── scenes/   — menu, game scene, scene manager
├── ecs/      — specs adapter, components, factory, groups, cursor
├── data/     — slots/objects, placement logic, map, pathfinding, door
├── ui/       — HUD, inventory, shop, settings, weather, text renderer
├── input/    — camera, cursor, interaction, desktop/touch abstraction
├── npc/      — ShopperNpc state machine and movement
├── audio/    — sound engine (rodio)
├── scripts/  — Lua bridge (config.lua, npc.lua)
├── save/     — worlds, manifest, settings.json
└── tests/    — unit tests (including docs checks)
```text

Assets live in `assets/`: textures, sounds, Lua scripts, maps and the font.
See [docs/en/architecture.md](docs/en/architecture.md) for the frame pipeline
and [docs/en/rendering.md](docs/en/rendering.md) for the layer order.

## License

GPL-3.0. See [LICENSE](LICENSE).
