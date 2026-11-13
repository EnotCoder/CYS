# Create your Shop (CYS)

> **Beta** — the game is in active beta. Features may change and bugs are
> expected.

![Gameplay](img/prev.png)

A 2D shop simulator written in Rust on top of `wgpu`. Build your own shop:
lay floors, walls and carpets, place racks, registers and decor, stock the
shelves, serve shopper NPCs and keep the business afloat.

## Build & run

```sh
cargo build --release
cargo run --release
```

Requires Rust edition 2021 and a GPU with Vulkan (Linux/Android) or Dx12
(Windows). For Android see [docs/build.md](docs/build.md).

## Controls

| Action | Input |
|---|---|
| Place / remove / interact | `LMB` or `F` |
| Cycle mode (move/interact/build/delete) | `Tab` or the mode icon |
| Open / close inventory | `E` |
| Change inventory tab | `←` / `→` or a tab icon |
| Change tab page | arrows beside the tab row |
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
- **Settings** — VSync, zoom speed, music and SFX, persisted in `data/settings.json`.
- **Balance and items in Lua** — `assets/scripts/config.lua`,
  `assets/scripts/items.lua` and `assets/scripts/npc.lua` drive timings, prices,
  the inventory catalogue and shopper decisions.

## Documentation

The full developer documentation lives in [`docs/`](docs/README.md).

| Topic | Page |
|---|---|
| Start here | [docs/architecture.md](docs/architecture.md) |
| Source map | [docs/source-map.md](docs/source-map.md) |
| Rendering | [docs/rendering.md](docs/rendering.md) |
| ECS | [docs/ecs.md](docs/ecs.md) |
| Gameplay | [docs/gameplay.md](docs/gameplay.md) |
| Saves & maps | [docs/world-format.md](docs/world-format.md) |
| Lua scripts | [docs/scripts.md](docs/scripts.md) |
| Assets | [docs/assets.md](docs/assets.md) |
| Build | [docs/build.md](docs/build.md) |
| Tests | [docs/testing.md](docs/testing.md) |
| Contributing | [docs/contributing.md](docs/contributing.md) |

The docs are checked by `cargo test`: every `.rs` file must appear in the
source map, and all relative links must resolve.

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
├── scripts/  — Lua bridge (config.lua, items.lua, npc.lua)
├── save/     — worlds, manifest, data/settings.json
└── tests/    — unit tests (including docs checks)
```

Assets live in `assets/`: textures, sounds, Lua scripts, maps and the font.
See [docs/architecture.md](docs/architecture.md) for the frame pipeline
and [docs/rendering.md](docs/rendering.md) for the layer order.

## License

GPL-3.0. See [LICENSE](LICENSE).
