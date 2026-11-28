# Source map

The source tree is split into three workspace crates:

- `engine/` — technical layer: wgpu, assets, input, ECS, text, UI components, audio and utilities.
- `game/` — gameplay layer: data, NPCs, scenes, game windows, saves, Lua scripts and tests.
- `app/` — build and orchestration: `App`, the winit loop, desktop and Android entry points.

Dependencies point only `app → game → engine`; `engine` never refers to `game`.

## engine (`engine/src/`)

### Root

- `engine/src/lib.rs` — public API of the `cys_engine` crate.
- `engine/src/profiler.rs` — frame timing measurements.
- `engine/src/util.rs` — shared technical utilities without gameplay dependencies.

### `core` (compatibility facade)

- `engine/src/core/mod.rs` — re-exports technical modules under the former `core::...` paths.

### `gfx` — rendering

- `engine/src/gfx/mod.rs` — graphics module root.
- `engine/src/gfx/buffers.rs` — GPU buffers and sprite vertices.
- `engine/src/gfx/constants.rs` — render constants, z-layers and gameplay constants (temporarily re-exported through `game::core`).
- `engine/src/gfx/init.rs` — wgpu and surface initialization.
- `engine/src/gfx/pipeline.rs` — pipelines and bind groups.
- `engine/src/gfx/render.rs` — frame rendering and presentation.
- `engine/src/gfx/sprite.rs` — basic sprite drawing without gameplay types.
- `engine/src/gfx/texture.rs` — texture loading and storage.

### `assets` — asset loading

- `engine/src/assets/mod.rs` — asset loading module root.
- `engine/src/assets/asset.rs` — project root lookup and file reading.
- `engine/src/assets/jobs.rs` — asset loading jobs.
- `engine/src/assets/shader_pack.rs` — shader packs.
- `engine/src/assets/texture_pack.rs` — texture packs and the texture layer provider.

### `input` — input

- `engine/src/input/mod.rs` — input module root.
- `engine/src/input/platform.rs` — desktop and touch input.
- `engine/src/input/zoom.rs` — camera zoom.

### `text`, `ui`, `audio`, `ecs`

- `engine/src/text/mod.rs` — technical text layer.
- `engine/src/text/text_input.rs` — text input field.
- `engine/src/ui/mod.rs` — UI component module root.
- `engine/src/ui/components.rs` — visual UI components.
- `engine/src/ui/fps.rs` — FPS indicator.
- `engine/src/audio/mod.rs` — sound through rodio.
- `engine/src/ecs/mod.rs` — basic ECS without gameplay systems.
- `engine/src/ecs/components.rs` — basic ECS components.
- `engine/src/ecs/factory.rs` — technical entity factory.

## game (`game/src/`)

### Root and facade

- `game/src/lib.rs` — public API of the `cys_game` crate.
- `game/src/core/mod.rs` — game core facade and gameplay provider initialization.
- `game/src/core/util.rs` — gameplay utilities that depend on ECS and data.
- `game/src/core/jobs.rs` — gameplay jobs built on `engine::assets::jobs`.
- `game/src/core/shader_pack.rs` — gameplay shader-pack facade.
- `game/src/core/texture_pack.rs` — gameplay texture-pack facade.

### `data` — data and placement

- `game/src/data/mod.rs` — Slot, Object, `ALL_OBJECTS` and the data facade.
- `game/src/data/placement.rs` — object placement rules.
- `game/src/data/door.rs` — doors.
- `game/src/data/map/mod.rs` — map loading and pathfinding entry point.
- `game/src/data/map/image_map.rs` — image-based map.
- `game/src/data/map/pathfinding.rs` — pathfinding.

### `ecs` — gameplay ECS

- `game/src/ecs/mod.rs` — gameplay ECS layer.
- `game/src/ecs/adapter/mod.rs` — adapter from gameplay entities to rendering.
- `game/src/ecs/adapter/render.rs` — rendering part of the adapter.
- `game/src/ecs/components.rs` — gameplay components.
- `game/src/ecs/cursor.rs` — ECS cursor.
- `game/src/ecs/group.rs` — entity groups.
- `game/src/ecs/placement.rs` — placement through ECS.
- `game/src/particles.rs` — particles.

### `input` — gameplay input

- `game/src/input/mod.rs` — gameplay input module root.
- `game/src/input/cursor.rs` — cursor and mouse actions.
- `game/src/input/interact.rs` — player interactions.

### `npc` — shoppers

- `game/src/npc/mod.rs` — `ShopperNpc` and the NPC module root.
- `game/src/npc/interactions.rs` — NPC interactions.
- `game/src/npc/movement.rs` — NPC movement.

### `scenes` — scenes

- `game/src/scenes/mod.rs` — scene module root.
- `game/src/scenes/menu_scene.rs` — main menu.
- `game/src/scenes/scene_manager.rs` — scene manager.
- `game/src/scenes/scene_trait.rs` — shared scene interface.
- `game/src/scenes/cutscene/mod.rs` — intro cutscene.
- `game/src/scenes/cutscene/renderer.rs` — cutscene renderer.
- `game/src/scenes/cutscene/stars.rs` — cutscene stars.
- `game/src/scenes/cutscene/timeline.rs` — cutscene timeline.
- `game/src/scenes/game/mod.rs` — game scene.
- `game/src/scenes/game/camera.rs` — camera panning and clamping.
- `game/src/scenes/game/day_night.rs` — day/night cycle.
- `game/src/scenes/game/hud.rs` — HUD.
- `game/src/scenes/game/inventory_input.rs` — inventory input.
- `game/src/scenes/game/level.rs` — level.
- `game/src/scenes/game/shoppers.rs` — shoppers in the game scene.
- `game/src/scenes/game/tutorial.rs` — tutorial.
- `game/src/weather_fx.rs` — level-scope weather effects.

### `ui` — gameplay windows

- `game/src/ui/mod.rs` — gameplay UI module root.
- `game/src/ui/anim.rs` — UI animations.
- `game/src/ui/inventory.rs` — inventory.
- `game/src/ui/mods.rs` — mods window.
- `game/src/ui/settings.rs` — settings.
- `game/src/ui/shop.rs` — shop.
- `game/src/ui/system.rs` — system UI actions.
- `game/src/ui/text_renderer.rs` — gameplay text renderer.
- `game/src/ui/weather.rs` — weather window.

### `scripts` — Lua

- `game/src/scripts/mod.rs` — Lua bridge.
- `game/src/scripts/config.rs` — script configuration.
- `game/src/scripts/items.rs` — Lua-defined items.
- `game/src/scripts/mods.rs` — mods.
- `game/src/scripts/npc.rs` — Lua-defined NPCs.

### `save` — saves

- `game/src/save/mod.rs` — worlds and save manifest.

### `tests` — unit tests

- `game/src/tests/mod.rs` — test registration.
- `game/src/tests/helpers.rs` — shared test helpers.
- `game/src/tests/anim_tests.rs` — animation tests.
- `game/src/tests/data_integrity.rs` — data integrity checks.
- `game/src/tests/docs.rs` — source-map and link checks.
- `game/src/tests/group_tests.rs` — group tests.
- `game/src/tests/input_tests.rs` — input tests.
- `game/src/tests/mods_tests.rs` — mod tests.
- `game/src/tests/npc_tests.rs` — NPC tests.
- `game/src/tests/particles_tests.rs` — particle tests.
- `game/src/tests/placement_rules.rs` — placement tests.
- `game/src/tests/shader_pack_tests.rs` — shader-pack tests.
- `game/src/tests/texture_pack_tests.rs` — texture-pack tests.

## app (`app/src/`)

- `app/src/lib.rs` — `App`, the render loop, winit event handling and `run()`.
- `app/src/main.rs` — desktop entry point.
- `app/src/android.rs` — Android entry point and bridge.
