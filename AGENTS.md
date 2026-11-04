# Project conventions

## Structure

12 main folders in `src/` (Godot-style):

- `src/main/` — entry point (`main.rs`) — is crate root via `[[bin]]` in Cargo.toml; `lib.rs` holds App, render loop, winit event handling; `android.rs` is the Android entry
- `src/core/` — kernel: wgpu pipeline (`buffers`,`init`,`pipeline`,`render`,`texture`,`shaders.wgsl`), constants, util, asset I/O
- `src/data/` — Slot, Object, ALL_OBJECTS (`mod.rs`), placement logic (`placement.rs`), map loading + pathfinding (`map/`), door (`door.rs`)
- `src/npc/` — ShopperNpc only
- `src/ecs/` — ECS adapter (`adapter/`), components, cursor, factory, group, placement, sprite
- `src/input/` — camera, cursor, interact, platform (desktop/touch)
- `src/scenes/` — menu_scene, game/ (GameScene), scene_manager, scene_trait
- `src/ui/` — components, fps, inventory, settings, shop, weather, anim, text_input, system, text_renderer
- `src/audio/` — sound (rodio)
- `src/scripts/` — Lua bridge (config, npc)
- `src/save/` — worlds, manifest, settings.json
- `src/tests/` — unit tests (tied in via `#[cfg(test)] mod tests;`) and docs checks

Documentation lives outside `src/`, in `docs/` (English only). Root `README.md`
is English.

## Texture paths

All asset paths start with `assets/`:

- Game map tiles: `assets/tex/map/{grass,floor,wall}.png`
- Decor objects: `assets/tex/decor/{regular,carpets,walldecor,outdoor,light}/...`
- UI textures: `assets/tex/ui/{active,cursor,checkbox,mode,slide,...}/...`
- Icons: `assets/tex/ui/icon_slots/icon_slots_object/{regular,carpets,walldecor,outdoor}/...`
- Dev tools: `assets/tex/dev_tools/{black,null}.png`
- Characters: `assets/tex/characters/{player,bob,sasha}/...`

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
