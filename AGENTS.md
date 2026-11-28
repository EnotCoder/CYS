# Project conventions

## Structure

The workspace has three crates; dependencies point only `app → game → engine`.

- `engine/` — technical layer: `gfx/` (wgpu pipeline, `shaders.wgsl`, buffers, constants, render, texture, sprite), `assets/` (asset I/O, jobs, shader and texture packs), `input/` (zoom, desktop/touch platform), `text/`, `ui/` (basic components, FPS), `audio/`, basic `ecs/`, `profiler.rs`, `util.rs`; `core/` is a compatibility facade over these modules
- `game/` — gameplay layer: `data/` (Slot, Object, ALL_OBJECTS, placement, map/, pathfinding, door), `npc/` (ShopperNpc, interactions, movement), `ecs/` (adapter/{mod,render}, components, cursor, group, placement), `input/` (cursor, interact), `scenes/` (menu, cutscene/, game/ with HUD, level, tutorial, weather FX, inventory input, manager, trait), `ui/` (inventory, mods, settings, shop, weather, anim, text_renderer, system), `scripts/` (config, items, npc, mods), `save/`, `particles.rs`, `weather_fx.rs`, `tests/`
- `app/` — binary and orchestration: `lib.rs` (App, render loop, winit events, `run()`), `main.rs` (desktop entry), `android.rs` (Android entry)

`game/src/core/` adds gameplay-specific util and pack facades on top of
`engine::core`; `game::core::init()` registers the mod texture-layer provider.
`EcsAdapter`, `TextRenderer`, `ui::system` and `ui::anim` stay in `game` because
they depend on gameplay types.

`docs/source-map.md` is the authoritative list of every `.rs` file, including
nested ones.

Documentation lives in `docs/` (English only). Root `README.md` is English.

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
