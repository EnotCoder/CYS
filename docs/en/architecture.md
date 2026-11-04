# Architecture (CYS)

[Русский](../ru/architecture.md) · English

The source tree follows the Godot convention: `src/` contains exactly
12 top-level module folders, each with a `mod.rs` that wires submodules
together. Everything outside `src/` is either an asset or a build artifact.

```
src/
├── main/
├── core/
├── scenes/
├── ecs/
├── data/
├── ui/
├── input/
├── npc/
├── audio/
├── scripts/
├── save/
└── tests/
```

Assets live in `assets/` (`assets/tex/`, `assets/sounds/`, `assets/scripts/`,
map files) and are not part of `src/`. On Android they are packed into the APK.

## Crate layout

Cargo builds two targets from the same crate:

| Target | Path | Role |
|---|---|---|
| `lib` (`rlib` + `cdylib`) | `src/main/lib.rs` | all game logic, `run()`, `android_main` |
| `bin` | `src/main/main.rs` | desktop entry point, calls `cys::run()` |

`lib.rs` declares the 12 modules with `#[path = "../<module>/mod.rs"]`, so the
folders keep their Godot-style shape while Cargo sees a flat module tree.
`main.rs` is intentionally tiny: desktop `main` only delegates to the library.

## One frame

`App` implements winit's `ApplicationHandler`:

```
new_events          → input.step()
resumed             → window + WgpuApp + surface
window_event        → input.process_window_event()
  RedrawRequested   → App::render()
about_to_wait       → input.end_step(); request_redraw()
suspended           → surface = None
```

`App::render` is the frame pipeline:

1. `scene.update(...)` — input, gameplay, animations. Returns `SceneAction`.
2. Apply the action: `Switch`, `Quit`, `VsyncToggle`, `None`.
3. Update IME state (text input on desktop and Android).
4. Tick the FPS counter and refresh the FPS label.
5. Write world uniforms (`Size`) and UI uniforms (`UiUniforms`).
6. `scene.sprites(...)` and `scene.lights(...)` — collect render data per layer,
   already culled by the visible bounds.
7. `core::render` — one pass for the map, one transparent pass for
   carpets/lights/decor/NPC/weather/cursor, one for UI.

## Scenes

`SceneManager` owns a single ECS world and a registry of scenes (`menu`,
`game`). `switch_to` stops music, clears the world and calls `on_enter` on the
new scene. Scenes implement the `Scene` trait:

| Method | Meaning |
|---|---|
| `on_enter` | build the scene's entities, UI and state |
| `update` | one tick; returns `SceneAction` |
| `sprites` | 8 sprite lists (map, carpet, light, decor, NPC, weather, cursor, UI) |
| `map_size`, `camera_offset`, `night_factor`, `lights`, `ui_size` | render uniforms |

`SceneAction` is the only channel from a scene back to `App`:
`Switch(&'static str)`, `Quit`, `VsyncToggle(bool)`, `None`.

## ECS and data flow

`EcsAdapter` wraps `specs::World` and registers every component and resource
once. Gameplay code never touches `specs` directly for rendering: it calls
`create_sprite`, `add_group_object`, `add_ui`, then reads flat
`SpriteRenderData` lists from `get_sprites_by_layer`.

`data::ALL_OBJECTS` is the single catalogue of placeable objects (name, size,
texture, price). `data::placement` decides whether a cell accepts an object;
`ecs::placement` (in `data`'s terms) sets and removes components.

## Input abstraction

`input::platform::InputSource` hides desktop and touch differences:

- `DesktopInput` forwards to `winit_input_helper`;
- `TouchInput` converts one-finger drag/tap and two-finger pinch into the same
  mouse/scroll events.

`App` stores `Box<dyn InputSource>` and passes it to scenes, so game code is
platform-agnostic. Keyboard/IME events are accumulated per frame; `step()` and
`end_step()` define the frame boundary.

## Persistence

`save/` owns the Minecraft-style world list:

- `worlds/manifest.json` — next id + metadata (`WorldMeta`);
- `worlds/world_{id}.json` — a full level snapshot;
- `settings.json` — VSync, zoom speed, music and SFX toggles.

`save::SELECTED_WORLD` is a static because `SceneAction` cannot carry
parameters; the menu writes it before `Switch("game")`. On Android the same
functions write to the app's internal data directory.

## Render layers

Lower `z` draws first. Values live in `core::constants`:

```
Z_MAP=0.0 → Z_CARPET=1.0 → Z_LIGHT=1.2 → Z_DECOR=1.5 (Z_DOOR)
→ Z_NPC=1.8 → Z_WEATHER=1.9 → Z_CURSOR=2.0
→ Z_UI=3.0 → Z_UI_TEXT=3.005 → tooltips → dialogs
```

## Reading order

1. `src/main/lib.rs` and `src/main/main.rs` — startup.
2. `src/scenes/scene_trait.rs` and `scene_manager.rs` — the frame contract.
3. `src/scenes/game/mod.rs` — the gameplay loop.
4. `src/core/render.rs` and `src/core/init.rs` — how pixels appear.
5. `src/data/mod.rs` — what the player can place.
