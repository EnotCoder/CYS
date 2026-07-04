# Project conventions

## Structure
- `src/main.rs` — App, render loop, winit event handling
- `src/constants.rs` — all game constants
- `src/data/` — Slot, Object, ALL_OBJECTS (`mod.rs`), placement logic (`placement.rs`)
- `src/map/` — map loading (`mod.rs`), pathfinding (`pathfinding.rs`)
- `src/npc/` — ShopperNpc only
- `src/ecs/` — ECS adapter (`adapter/`), components, cursor, factory, group, placement, sprite
- `src/input/` — camera, cursor, interact
- `src/scene/` — menu_scene, game/ (GameScene), scene_manager, scene_trait
- `src/ui/` — components, fps, settings, system, text_renderer

## Texture paths
- Game map tiles: `tex/map/{grass,floor,wall}.png`
- Decor objects: `tex/decor/{regular,carpets,walldecor,outdoor}/...`
- UI textures: `tex/ui/{active,cursor,checkbox,mode,slide}/...`
- Icons: `tex/ui/icon_slots/{regular,carpets,walldecor,outdoor}/...`
- Dev tools: `tex/dev_tools/{black,null}.png`
- Characters: `tex/characters/player/{player,player_walk_1,player_walk_2}.png`

## Code style
- No comments on trivial code
- Prefer `replaceAll` over single edits when renaming
- Import grouped: `use crate::{A, data::B}`
- Keep `impl Type` blocks together in one file (trait impls cannot be split across submodules)
- When refactoring: first move files, then update all use paths, then build

## Rendering layers (z-order)
- Z_MAP = 0.0, Z_CARPET = 1.0, Z_DECOR = 1.5, Z_NPC = 2.0, Z_CURSOR = 2.5, Z_UI = 3.0

## wgpu 30.0.0 notes
- ApplicationHandler, Instance by value, Surface<'static>
- CurrentSurfaceTexture is an enum, queue.present() takes no args
- AMD RADV: max_uniform_buffer_binding_size = 65536
