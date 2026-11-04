# Source map (`src/`)

Every `.rs` file: what it implements and which role it plays in the game. The
folder layout follows the Godot convention, see [architecture.md](architecture.md).

Paths are written in full: the `src/tests/docs.rs` test fails if any `.rs` file
is missing from this map.

---

## `src/main/`

- **`src/main/main.rs`** — desktop `[[bin]]` entry point; delegates everything
  to `cys::run()` from the library.
- **`src/main/lib.rs`** — crate root: declares 12 modules via `#[path]`, defines
  `App` (window, `WgpuApp`, surface, scene, input, FPS, IME), `render` (full
  frame), `ApplicationHandler`, desktop `run()` and the Android module.
- **`src/main/android.rs`** — `android_main`, hiding the system UI, the JNI
  bridge to `EditText` for world names, `poll_text_input` and
  `sync_text_input`.

---

## `src/core/` — wgpu kernel

- **`src/core/mod.rs`** — wires `asset`, `buffers`, `init`, `pipeline`,
  `render`, `texture`, `constants`, `util` and re-exports the main types.
- **`src/core/asset.rs`** — unified file access: `std::fs` on desktop,
  `AAssetManager` on Android (strips the `assets/` prefix); saves and settings
  go to the app's internal directory; `list_dir` for sounds.
- **`src/core/constants.rs`** — every constant: z-layers, camera and zoom,
  window and UI sizes, texture paths, inventory lists, `MAX_DYNAMIC_SPRITES`,
  `QUAD_INDICES`, render service values.
- **`src/core/buffers.rs`** — shader-compatible `Vertex` and `Uniforms`,
  `DepthBuffer`, `init_buffers` (bind group layouts, dynamic storage buffer
  with offsets, texture and sampler), depth-stencil states.
- **`src/core/init.rs`** — `WgpuApp::new`: instance → surface → adapter →
  device/queue, `Size` and `UiUniforms` buffers, dynamic storage buffer, both
  pipelines, surface configuration; global `UI_UNIFORMS`.
- **`src/core/pipeline.rs`** — two pipelines: opaque (`create_render_pipeline`,
  alpha blending, depth LessEqual) and transparent
  (`create_transparent_pipeline`, Always) plus the vertex layout.
- **`src/core/render.rs`** — `render` by layers: map (clear), transparent pass,
  UI; `render_group` — sprite cache, batching, `write_buffer`, dynamic offsets;
  passes light sources.
- **`src/core/shaders.wgsl`** — vertex and fragment shaders: quads, camera,
  lighting, night dimming, UI space.
- **`src/core/texture.rs`** — `Texture` (texture + view + sampler): from
  PNG/JPEG bytes, from disk (`from_path`, falls back to `null.png`), from raw
  RGBA (`from_rgba`, text and UI).
- **`src/core/util.rs`** — `sprite_cache_key`, `ndc_to_world`, `inventory_index`,
  `slot_icon_path`, `ui_fit_scale`, `ui_top_offset`/`ui_bottom_offset`,
  `ease_out_back`.

---

## `src/data/` — data, map, placement

- **`src/data/mod.rs`** — `Slot`/`Object`, the static `ALL_OBJECTS` catalogue,
  `make_slot`, `get_slot_vec`, `object_price`, `shop_item_names`; re-exports
  placement rules; declares `door`, `map`, `placement`.
- **`src/data/placement.rs`** — `add`/`remove` rules: cell categories, wall
  recomputation (`refresh_walls_around`, `recompute_wall_token`,
  `revert_to_grass`), economy (price and half refund), `FoodStorage`,
  `FenceComponent`, `BasementPlaced`, basement checks.
- **`src/data/door.rs`** — `tick_door`: the door opens when a shopper
  (`ShopperTag`) is near and closes after they leave; `DoorProgress`, 2×2 atlas
  frames, smooth NPC fade inside the doorway.
- **`src/data/map/mod.rs`** — loads `map.txt`/`basement.txt` into the ECS
  (`load_map_to_ecs`, `load_basement_to_ecs`, `spawn_map_tile`), wall/floor/
  grass/walkable sets, `shopper_spawn_point`, `token_to_texture`, `door_cells`,
  `door_frame_for_cell`.
- **`src/data/map/pathfinding.rs`** — A*: `Node` (cell, 4 neighbours, Manhattan
  heuristic), `find_path` with a binary heap and path reconstruction,
  `from_world`/`to_world`.

---

## `src/ecs/` — specs and sprites

- **`src/ecs/mod.rs`** — declares `adapter`, `components`, `cursor`, `factory`,
  `group`, `placement`, `sprite`; re-exports the adapter and `Sprite`.
- **`src/ecs/adapter/mod.rs`** — `EcsAdapter`: owns the `World`, sprite cache,
  textures and map position sets; registers components and resources, provides
  `add_ui`/`add_ui_sized`, world clearing and `SpriteRenderData`.
- **`src/ecs/adapter/render.rs`** — `get_sprites_by_layer` (8 z-layers culled by
  visible bounds), `update_object_textures` (box/rack frames by food count),
  `update_fence_textures` (fence by neighbours), `update_sprite_alpha`,
  `update_transform_position`.
- **`src/ecs/components.rs`** — components and resources: `Transform`,
  `SpriteComponent`, `GroupComponent`, `GroupInfoResource`, `Rotation`,
  `ObjectTag`, `FoodStorage`, `ShopperTag`; `TotalFood`, `Money`, `BusyCassas`,
  `BasementPlaced`, `ShopOwned`, `FenceComponent`, `Season`, `DoorState`,
  `DoorProgress`.
- **`src/ecs/cursor.rs`** — `add_cursor`, `update_cursor_preview` (translucent
  object ghost, atlas frames, error markers), `clear_cursor_preview`.
- **`src/ecs/factory.rs`** — `create_sprite`: the only way to create a sprite
  (`Transform` + `SpriteComponent` from position, texture, frame, scale, alpha).
- **`src/ecs/group.rs`** — multi-cell objects: `add_group_object` (one entity per
  tile with an atlas frame offset), `delete_group`, `find_group_at_position`
  (decor beats light, light beats carpet).
- **`src/ecs/placement.rs`** — `can_place_at`: categories require matching
  cells (carpet/light/decor — floor, wall decor — wall, outdoor/flowers —
  grass) and forbid placing an object over the same category.
- **`src/ecs/sprite.rs`** — `Sprite`: a ready quad (vertices, atlas frame with
  `TEXEL_EPSILON`, buffers, texture bind group); `new` for atlases,
  `from_texture` for text/UI, `split_sized_path` for `tex.png@WxH` paths.

---

## `src/npc/` — shopper

- **`src/npc/mod.rs`** — `ShopperNpc`: state machine (`GoingToRack` →
  `GoingToCassa` → `AtCassa` → `GoingToCandies` → `AtCandies` →
  `GoingToExit`), fade-in spawn, Lua API, `update` (`npc.lua` script or
  `update_fallback` in Rust).
- **`src/npc/interactions.rs`** — `take_food`/`take_candy`, `cassa_exists`,
  `rack_exists`, `candy_exists`, `find_any_cassa`, `reroute_to_cassa`, sounds
  and textures.
- **`src/npc/movement.rs`** — `spawn_path_node`, `set_idle`/`set_walk`,
  `start_path` (A* plus `exit_target`), `walk_toward` (step, animation,
  rotation), `walk_to_exit`.

---

## `src/scenes/` — scenes

- **`src/scenes/mod.rs`** — declares `scene_trait`, `scene_manager`,
  `menu_scene`, `game`; re-exports `Scene`, `SceneManager`, `MenuScene`,
  `GameScene`.
- **`src/scenes/scene_trait.rs`** — the `Scene` trait (`on_enter`, `on_exit`,
  `update`, `sprites`, `map_size`, `camera_offset`, `night_factor`, `lights`,
  `ui_size`) and `SceneAction`.
- **`src/scenes/scene_manager.rs`** — `SceneManager`: `menu`/`game` registry,
  one ECS world, `switch_to` (stop music, clear, `on_enter`), `update_fps`,
  `clear_ecs_world`.
- **`src/scenes/menu_scene.rs`** — `MenuScene`: background map, logo,
  Play/Quit buttons, world pick/create/delete with confirmation, hover
  highlight, Space/click to start, Escape to quit.
- **`src/scenes/game/mod.rs`** — `GameScene`: main loop — input, rent,
  bankruptcy, settings, food regen, pop animations, day/night, shoppers,
  weather, tutorial, shop, hover/tooltip, autosave, level switching.
- **`src/scenes/game/camera.rs`** — `update_camera`: movement only in `move`
  mode (mouse/finger drag or arrow keys), clamped to the map's visible area.
- **`src/scenes/game/day_night.rs`** — `DayNightCycle`: `tick`, `factor`
  (0..1 dimming with dawn and dusk), `time_string` (`HH:MM`).
- **`src/scenes/game/hud.rs`** — `GameHud`: "Loading...", object hint,
  Food/Money counters, clock (at most once per second), item tooltip, info
  panel; text is recreated only when it changes.
- **`src/scenes/game/inventory_input.rs`** — inventory input: `E` toggles the
  grid, category tabs, grid clicks, move to the active slot, hotbar selection.
- **`src/scenes/game/level.rs`** — levels: `save_current_level` into
  `LevelState`, `load_level` (shop ↔ basement from cache), `rebuild_ui`,
  `save_to_disk`/`load_from_disk` (`worlds/world_{id}.json`), basement exit.
- **`src/scenes/game/shoppers.rs`** — `ShopperManager`: `spawn_shopper`
  (random rack/register/candy target, three texture sets), `set_active`, `tick`
  (spawn timers, limit, despawn, cooldown).
- **`src/scenes/game/tutorial.rs`** — `Tutorial`: tutorial panel, `open`/`close`,
  button and panel hit tests, gentle sine sway.
- **`src/scenes/game/weather_fx.rs`** — `WeatherFx`: rain, snow and leaf
  particles, `Effect`, sprite spawn/teardown, deterministic `rand01`.

---

## `src/ui/` — interface

- **`src/ui/mod.rs`** — declares `anim`, `components`, `fps`, `inventory`,
  `settings`, `shop`, `system`, `text_input`, `text_renderer`, `weather`;
  re-exports components and system helpers.
- **`src/ui/components.rs`** — element descriptions: `Panel`, `Button`,
  `Checkbox` (with `sprite_key`), `Slider` (track, thumb, `dragging`).
- **`src/ui/system.rs`** — UI creation and deletion: `ndc_to_ui`, `is_inside`,
  `is_clicked`, factories and destructors, `slider_drag`,
  `update_slider_thumb`.
- **`src/ui/inventory.rs`** — `Inventory`: grid and tabs, `items`, `switch_tab`,
  grid clicks, move to the hotbar.
- **`src/ui/settings.rs`** — settings panel: VSync (`vsync_toggled`), zoom
  speed slider (`zoom_speed_changed`), music/SFX, `open`/`close`,
  `handle_input`, `apply_saved`.
- **`src/ui/shop.rs`** — `Shop`: scrollable catalogue with a scrollbar,
  `ShopItem`, `ConfirmAction`, purchase confirmation dialog, `refresh`,
  `row_clicked`, `scroll_input`.
- **`src/ui/weather.rs`** — `Weather`: season panel (4 buttons), `selected`,
  `season_changed`, `open`/`close`, `handle_input`, highlight.
- **`src/ui/anim.rs`** — `PanelAnim`: panel open/close, `AnimPhase`/`AnimEvent`,
  `start_open`/`start_close`/`cancel`, `tick`, capture and reset of initial
  transforms; timings 0.28/0.22 s.
- **`src/ui/text_input.rs`** — `TextInput` and global `TEXT_INPUT`: active flag,
  focus, IME preedit, `push`, `backspace`, `enter`, `take`; used for world names.
- **`src/ui/fps.rs`** — `FpsCounter`: recomputes FPS once per second from the
  frame count.
- **`src/ui/text_renderer.rs`** — `TextRenderer`: `ab_glyph` rasterisation into
  an RGBA texture (outline and fill), text sprites, cache keyed by
  text+size+colour; `add_text`, `set_text`, `add_text_z`.

---

## `src/input/` — input

- **`src/input/mod.rs`** — declares `camera`, `cursor`, `interact`, `platform`;
  `do_input` — the per-frame input entry: zoom, cursor, clicks and `F`, UI-zone
  exclusion, `Tab` mode switch, validity and preview, slot return.
- **`src/input/camera.rs`** — `handle_zoom`: wheel, `K`/`L`, range
  `ZOOM_MIN..ZOOM_MAX`.
- **`src/input/cursor.rs`** — `handle_mouse_movement` (NDC → world → cell,
  clamping, per-tile delay), `update_cursor_validity` (ok/error texture),
  `update_cursor_preview`; `move` mode leaves the map untouched.
- **`src/input/interact.rs`** — `try_interact` (box — sell food, rack — refill,
  basement — level switch, arcade — refund 1), `cycle_mode`, `do_interact`,
  `next_mode`.
- **`src/input/platform.rs`** — `InputSource` (unified interface),
  `DesktopInput` over `winit_input_helper`, `TouchInput` (tap, drag, pinch),
  keyboard, IME and text event handling.

---

## `src/audio/`

- **`src/audio/mod.rs`** — rodio `AudioEngine`: `sounds/` cache, singleton via
  `OnceLock`/`Mutex`; `play`, `play_music`, `play_ambient`, `stop_music`,
  `set_music_enabled`, `set_sfx_enabled`, `init`.

---

## `src/scripts/` — Lua bridge

- **`src/scripts/mod.rs`** — declares `config` and `npc`.
- **`src/scripts/config.rs`** — `BalanceConfig`: reads
  `assets/scripts/config.lua` through mlua, falls back to defaults on error;
  timings, prices, capacities, economy; `publish_to_lua`.
- **`src/scripts/npc.rs`** — `NpcScript`: calls `npc.lua` from Rust, packs NPC
  state into a Lua table, exposes primitives (`walk`, `take_food`, `start_path`,
  `busy`, `add_money`, `find_any_cassa`), copies `state`/`timer` back, falls
  back when the script is missing.

---

## `src/save/`

- **`src/save/mod.rs`** — worlds: `WorldMeta`, `WorldManifest`, `list_worlds`,
  `create_world_with_name`, `unique_world_name`, `touch_world`, `delete_world`,
  `WorldSelection`/`SELECTED_WORLD`; settings `GameSettings`, `load_settings`,
  `save_settings`, global `SETTINGS`.

---

## `src/tests/`

- **`src/tests/mod.rs`** — declares the test modules and holds general checks:
  modes, text input, pure functions, constants.
- **`src/tests/helpers.rs`** — shared fixtures: ECS without a GPU, object
  placement, `FakeInput` (`InputSource` implementation), UI-to-pixel mapping.
- **`src/tests/data_integrity.rs`** — invariants of constants, the object
  catalogue, balance, day/night, sprite cache and settings.
- **`src/tests/placement_rules.rs`** — `can_place_at`, `add`/`remove`: cell
  categories, conflicts, economy, basement, walls.
- **`src/tests/group_tests.rs`** — multi-cell objects: layout, atlas frames,
  layers, position lookup, group deletion.
- **`src/tests/npc_tests.rs`** — shopper: states, movement, food and candy,
  registers, routing, fade-out, door.
- **`src/tests/input_tests.rs`** — input through `FakeInput`: zoom, UI hit
  tests, slider, shop scrolling, world name, door fade, weather.
- **`src/tests/anim_tests.rs`** — `PanelAnim`: open, close, cancel, text
  sprites.
- **`src/tests/docs.rs`** — documentation checks: this map's completeness, live
  links and images.
