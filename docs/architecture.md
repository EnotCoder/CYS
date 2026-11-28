# Architecture (CYS)

The source tree is a Cargo workspace with three crates:

```
engine/     technical layer: gfx, assets, input, text, ui, audio, ecs, util
game/       gameplay layer: data, npc, scenes, ecs, ui, input, scripts, save, tests
app/        App, winit loop, desktop and Android entry points
```

Dependencies point only `app → game → engine`; `engine` never refers to `game`.
`engine/src/core/` is a compatibility facade over the technical modules, and
`game/src/core/` adds gameplay-specific util and pack facades. `EcsAdapter`,
`TextRenderer`, `ui::system` and `ui::anim` stay in `game` because they depend
on gameplay types. The existing render API is preserved: scenes still receive
`wgpu::Device`/`Queue` handles through `engine` types. Replacing that with a
`draw_sprite`-style engine API is a separate step. Nested areas include
`game/src/scenes/game/` (HUD, level,
tutorial, weather FX, inventory input), `game/src/scenes/cutscene/` (intro),
`game/src/ecs/adapter/` and `game/src/data/map/`. The authoritative file-by-file
list is [source-map.md](source-map.md).

Assets live in `assets/` (`assets/tex/`, `assets/sounds/`, `assets/scripts/`,
map files) and are not part of the source crates. On Android they are packed
into the APK.
Player content lives only under `assets/` on both platforms:
`assets/texture_pack/`, `assets/shaders/` and `assets/mods/`. Desktop reads the
same paths from the checkout or installed assets, and writes player overrides
to the user data directory; Android reads them from the APK. See
[texture-packs.md](texture-packs.md), [shaders.md](shaders.md) and
[mods.md](mods.md).

## Crate layout

| Crate | Package | Role |
|---|---|---|
| `engine/` | `cys-engine` | wgpu, assets, input, text, basic UI/ECS, audio, utilities |
| `game/` | `cys-game` | gameplay, scenes, gameplay ECS adapter, UI windows, scripts, saves, tests |
| `app/` | `cys` | `App`, render loop, winit events, `run()`, `android_main`, desktop binary |

`app/` builds a `cdylib` + `rlib` library and a `cys` binary from
`app/src/lib.rs` and `app/src/main.rs`. The desktop `main.rs` is intentionally
tiny: it only calls
`cys::run()`. `run()` initializes mods and gameplay providers with
`cys_game::scripts::mods::init()` and `cys_game::core::init()` before the first
frame, then starts the winit event loop.

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

1. `cys_engine::core::jobs::poll()` — collect results from background workers
   on the main thread (decoded rasters, completed saves).
2. `scene.update(...)` — input, gameplay, animations. Returns `SceneAction`.
3. Apply the action: `Switch`, `Quit`, `VsyncToggle`, `None`.
4. Update IME state (text input on desktop and Android).
5. Tick the FPS counter and refresh the FPS label.
6. `scene.sprites(...)` and `scene.lights(...)` — collect render data per layer,
   already culled by the visible bounds.
7. Write world uniforms (`Size`) and UI uniforms (`UiUniforms`).
8. `cys_engine::core::render` — one pass for the map, one transparent pass for
   carpets/lights/decor/NPC/weather (including placement particles)/cursor,
   one for UI.

With `CYS_PROFILE=1`, stages `update`, `sprites`, `uniforms`, `render_cache`
and `render_encode` are timed and reported to stderr once per second. The
profiler is inert by default and never touches game state.

### Background jobs

The optional `mt` Cargo feature enables two desktop-only threads: `cys-decode`
decodes texture files into RGBA bytes, and `cys-save` writes world JSON.
Workers never touch ECS, Lua, winit, wgpu or audio; they only produce bytes,
and `App::render` decides when to consume them. Decode results carry a
generation tag that `EcsAdapter::apply_texture_pack` invalidates, so a stale
result from an older pack is discarded. Android always uses the sequential
path, even if the feature is enabled.

## Scenes

`SceneManager` owns a single ECS world and a registry of scenes (`menu`,
`intro`, `game`). `switch_to` stops music, clears the world and calls
`on_enter` on the new scene. Scenes implement the `Scene` trait:

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

- `data/worlds/manifest.json` — next id + metadata (`WorldMeta`);
- `data/worlds/world_{id}.json` — a full level snapshot;
- `data/settings.json` — VSync, zoom speed, music and SFX toggles.

`save::SELECTED_WORLD` is a static because `SceneAction` cannot carry
parameters; the menu writes it before `Switch("game")`. On Android the same
functions write to the app's internal data directory.

## Render layers

Lower `z` draws first. Values live in `cys_engine::core::constants` and are
re-exported as `cys_game::core::constants`:

```
Z_MAP=0.0 → Z_CARPET=1.0 → Z_LIGHT=1.2 → Z_DECOR=1.5 (Z_DOOR)
→ Z_NPC=1.8 → Z_WEATHER=1.9 → Z_CURSOR=2.0
→ Z_UI=3.0 → Z_UI_TEXT=3.005 → tooltips → dialogs
```

## Reading order

1. `app/src/lib.rs` and `app/src/main.rs` — startup.
2. `game/src/scenes/scene_trait.rs` and `scene_manager.rs` — the frame contract.
3. `game/src/scenes/game/mod.rs` — the gameplay loop.
4. `engine/src/gfx/render.rs` and `init.rs` — how pixels appear.
5. `game/src/data/mod.rs` — what the player can place.
