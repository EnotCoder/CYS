# Create your Shop (CYS)

> **Beta** — the game is in active beta. Features may change and bugs are expected.

2D sandbox game written in Rust on wgpu. Build and manage your own shop: place
flooring, decor and walls, stock racks, serve shopper NPCs and keep the
business from going bankrupt.

Uses [winit](https://docs.rs/winit) for windows/input, [specs](https://docs.rs/specs)
for ECS, and Lua ([mlua](https://docs.rs/mlua)) for balancing.

## Build & run

```sh
cargo build --release
cargo run --release
```

Requires Rust edition 2021; a wgpu 30 compatible GPU (Vulkan on Linux).

## Controls

| Action                          | Input                          |
| ------------------------------- | ------------------------------ |
| Place / remove / interact       | `LMB` or `F`                   |
| Cycle mode (move/interact/build/del) | `Tab` or click the mode icon |
| Open / close inventory          | `E`                            |
| Select hotbar slot              | Click the slot                 |
| Toggle shop open/closed         | Click the active icon          |
| Zoom                            | `Scroll`, `K` / `L`, pinch     |
| Move camera (mode `move`)       | Drag mouse/finger, Arrow keys |
| Settings (pauses the game)      | `Esc` / Android Back button    |
| Quick save                      | `Ctrl+S` (load via world menu) |
| Enter basement                  | Click the basement stairs      |
| Back to menu                    | `R` (when bankrupt)            |

On Android: tap = click, one-finger drag moves the camera only in `move` mode,
two-finger pinch = zoom, system Back button = close panels / open the pause
menu (settings), which also pauses the game when opened. `move` is the default
mode; switch to interact/build/delete to work with the world.

## Features

- Build mode: floor, walls, carpets, outdoor decor, flowers
- Objects: racks, box, cash registers, candy showcase, arcade machines, fences
- Shopper NPCs with A* pathfinding, queueing and purchases
- Day/night cycle with dynamic 2D point lighting
- Mini-economy: prices, rent, bankruptcy, open/closed state
- Basement level with two-way transitions
- World slots: create new worlds, delete with confirmation (Minecraft-style)
- Pause: opening Settings freezes the world (day/night, rent, shoppers, weather)
- Settings with sound toggles (music / SFX); all settings persist in
  `settings.json` across launches (VSync, zoom speed, sound)
- Balance and NPC logic configurable in `scripts/config.lua`, `scripts/npc.lua`

## Architecture

```
src/
├── main/     — entry point, App, winit event loop
├── core/     — wgpu kernel: pipeline, textures, render, lighting, shaders
├── scenes/   — Menu, Game scene, scene manager
├── ecs/      — specs adapter, components, factory, groups
├── data/     — slots/objects, placement logic, map & pathfinding
├── ui/       — HUD, inventory, settings, text renderer
├── input/    — camera, cursor, interaction
├── npc/      — Shopper NPC logic
├── audio/    — sound (rodio)
├── scripts/  — Lua bridge (config.lua, npc.lua)
├── doc/      — documentation
└── tests/    — unit tests
```

Details: `src/doc/ARCHITECTURE.md`, `src/doc/RS.md`, `src/doc/SCRIPTS.md`.

## License

GPL-3.0. See [LICENSE](LICENSE).