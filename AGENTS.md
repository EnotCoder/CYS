# Project conventions

## Structure

12 main folders in `src/` (Godot-style), with nested subfolders where a folder
has grown:

- `src/main/` — entry point (`main.rs`) — is crate root via `[[bin]]` in Cargo.toml; `lib.rs` holds App, render loop, winit event handling; `android.rs` is the Android entry
- `src/core/` — kernel: wgpu pipeline (`buffers`, `init`, `pipeline`, `render`, `texture`, `shaders.wgsl`), constants, util, asset I/O, shader and texture packs
- `src/data/` — Slot, Object, ALL_OBJECTS (`mod.rs`), placement logic (`placement.rs`), map loading + pathfinding (`map/{mod,image_map,pathfinding}.rs`), door (`door.rs`)
- `src/npc/` — ShopperNpc (`mod.rs`), interactions (`interactions.rs`), movement (`movement.rs`)
- `src/ecs/` — ECS adapter (`adapter/{mod,render}.rs`), components, cursor, factory, group, particles, placement, sprite
- `src/input/` — zoom camera, cursor, interact, platform (desktop/touch); pan/clamp camera lives in `src/scenes/game/camera.rs`
- `src/scenes/` — menu_scene, cutscene/ (intro), game/ (GameScene with hud, level, tutorial, weather FX and inventory input), scene_manager, scene_trait
- `src/ui/` — components, fps, inventory, mods, settings, shop, weather, anim, text_input, text_renderer, system
- `src/audio/` — sound (rodio)
- `src/scripts/` — Lua bridge (config, items, npc, mods)
- `src/save/` — worlds, manifest, data/settings.json
- `src/tests/` — unit tests (tied in via `#[cfg(test)] mod tests;`) and docs checks

`docs/source-map.md` is the authoritative list of every `.rs` file, including
nested ones.

Documentation lives outside `src/`, in `docs/` (English only). Root `README.md`
is English.

## Texture paths

All asset paths start with `assets/`:

- Game map tiles: `assets/tex/map/{grass,floor,wall}.png`
- Decor objects: `assets/tex/decor/{regular,carpets,walldecor,outdoor,light}/...`
- UI textures: `assets/tex/ui/{active,cursor,checkbox,mode,slide,...}/...`
- Icons: `assets/tex/ui/icon_slots/icon_slots_object/{regular,carpets,walldecor,outdoor}/...`
- Dev tools: `assets/tex/dev_tools/{black,null}.png`
- Characters: `assets/tex/characters/{player,bob,sasha,mark_salesman,peter_businessman}/...`

## Code style

- No comments on trivial code
- Prefer `replaceAll` over single edits when renaming
- Import grouped: `use crate::{A, data::B}`
- Keep `impl Type` blocks together in one file (trait impls cannot be split across submodules)
- When refactoring: first move files, then update all use paths, then build

## Rendering layers (z-order)

`Z_MAP=0.0 → Z_CARPET=1.0 → Z_LIGHT=1.2 → Z_DECOR=1.5 (Z_DOOR)
→ Z_NPC=1.8 → Z_WEATHER=1.9 → Z_CURSOR=2.0 → Z_UI=3.0`
(UI text 3.005, tooltips 3.01/3.02, dialogs 3.03/3.035/3.04)

## Documentation

- Documentation is English only and lives in `docs/`.
- A new/renamed/removed `.rs` file must be added to `docs/source-map.md`.
- Run `cargo test docs` after editing docs: it checks source-map coverage and
  relative links.
- Update the matching page for balance (`gameplay.md`, `scripts.md`), save
  fields (`world-format.md`), assets (`assets.md`) and layers (`rendering.md`).

## wgpu 30.0.0 notes

- ApplicationHandler, Instance by value, Surface<'static>
- CurrentSurfaceTexture is an enum, queue.present() takes no args
- AMD RADV: max_uniform_buffer_binding_size = 65536
