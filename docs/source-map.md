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
  `App` (window, `WgpuApp`, surface, scene, input, FPS, IME), initialises mods
  before balance/scripts load, applies the saved texture pack and shader before
  the first frame, handles shader reload actions, `render` (full frame),
  `ApplicationHandler`, desktop `run()` and the Android module.
- **`src/main/android.rs`** — `android_main`, hiding the system UI, the JNI
  bridge to `EditText` for world names, `poll_text_input` and
  `sync_text_input`.

---

## `src/core/` — wgpu kernel

- **`src/core/mod.rs`** — wires `asset`, `buffers`, `init`, `pipeline`,
  `render`, `shader_pack`, `texture`, `texture_pack`, `constants`, `util` and
  re-exports the main types.
- **`src/core/asset.rs`** — unified file access: `std::fs` on desktop,
  `AAssetManager` on Android (strips the `assets/` prefix); saves and settings
  go to the app's internal directory; `list_dir` for sounds.
- **`src/core/constants.rs`** — every constant: z-layers, camera and zoom,
  window and UI sizes, texture paths (including the Mark portrait and walker),
  inventory lists, `MAX_DYNAMIC_SPRITES`, `QUAD_INDICES`, render service values.
- **`src/core/buffers.rs`** — shader-compatible `Vertex` and `Uniforms`,
  `DepthBuffer`, `init_buffers` (bind group layouts, dynamic storage buffer
  with offsets, texture and sampler), depth-stencil states.
- **`src/core/init.rs`** — `WgpuApp::new`: instance → surface → adapter →
  device/queue, `Size` and `UiUniforms` buffers, dynamic storage buffer, both
  pipelines, surface configuration; `reload_shader` rebuilds both pipelines with
  validation scopes and vanilla fallback; global `UI_UNIFORMS`.
- **`src/core/pipeline.rs`** — two pipelines: opaque (`create_render_pipeline`,
  alpha blending, depth LessEqual) and transparent
  (`create_transparent_pipeline`, Always) plus the vertex layout.
- **`src/core/render.rs`** — `render` by layers: map (clear), transparent pass,
  UI; `render_group` — sprite cache, batching, `write_buffer`, dynamic offsets;
  passes light sources.
- **`src/core/shaders.wgsl`** — vertex and fragment shaders: quads, camera,
  lighting, night dimming, UI space.
- **`src/core/shader_pack.rs`** — player WGSL shaders: discovery of
  `shaders/<id>/`, `shader.json` metadata and custom file, source loading,
  activation, error state and the embedded vanilla source.
- **`src/core/texture.rs`** — `Texture` (texture + view + sampler): from
  PNG/JPEG bytes, from disk (`from_path` resolves the active pack, retries the
  vanilla path, then falls back to `null.png`), from raw RGBA (`from_rgba`,
  text and UI).
- **`src/core/texture_pack.rs`** — player texture packs: discovery of
  `texture_pack/<id>/`, `pack.json` metadata/aliases, safe path resolution with
  vanilla fallback, live activation, object/NPC animation overrides and mod
  texture overlay layers above the active pack.
- **`src/core/util.rs`** — `sprite_cache_key`, `ndc_to_world`, `inventory_index`,
  `slot_icon_path`, `slot_icon_size`, `ui_fit_scale`,
  `ui_top_offset`/`ui_bottom_offset`, `ease_out_back`, `rand01`.

---

## `src/data/` — data, map, placement

- **`src/data/mod.rs`** — `Slot`/`Object`, `ItemMeta`/`ItemKind`, `ItemCatalog`,
  `ALL_OBJECTS`/`catalog` loaded from `assets/scripts/items.lua` with a built-in
  Rust fallback, `make_slot`, `get_slot_vec`, `object_price`, `shop_item_names`,
  `inventory_tab_names`, `item_kind`; re-exports placement rules;
  declares `door`, `map`, `placement`.
- **`src/data/placement.rs`** — `add`/`remove` rules: cell categories, wall
  recomputation (`refresh_walls_around`, `recompute_wall_token`,
  `revert_to_grass`), economy (price and half refund), `FoodStorage`,
  `FenceComponent`, `BasementPlaced`, basement checks and `object_placed` /
  `object_removed` mod events. Successful `add`/`remove` also push a
  `PlacementFxEvent` with the item's sampled colour into
  `EcsAdapter::pending_placement_fx`.
- **`src/data/door.rs`** — `tick_door`: the door opens when a shopper
  (`ShopperTag`) is near and closes after they leave; `DoorProgress`, 2×2 atlas
  frames, smooth NPC fade inside the doorway.
- **`src/data/map/image_map.rs`** — reads `assets/map/*.png` as one 16 px/cell
  image, matches each tile against the atlas frames (including flips) to
  recover walkable/floor/wall categories, caches the parsed grid and applies
  the 2×2 door override.
- **`src/data/map/mod.rs`** — loads the image map and `basement.txt` into the
  ECS (`load_map_to_ecs`, `load_basement_to_ecs`, `spawn_map_tile`,
  `set_map_season`), wall/floor/grass/walkable sets, `shopper_spawn_point`,
  `token_to_texture`, `door_cells`, `door_frame_for_cell`.
- **`src/data/map/pathfinding.rs`** — A*: `Node` (cell, 4 neighbours, Manhattan
  heuristic), `find_path` with a binary heap and path reconstruction,
  `from_world`/`to_world`.

---

## `src/ecs/` — specs and sprites

- **`src/ecs/mod.rs`** — declares `adapter`, `components`, `cursor`, `factory`,
  `group`, `particles`, `placement`, `sprite`; re-exports the adapter and `Sprite`.
- **`src/ecs/adapter/mod.rs`** — `EcsAdapter`: owns the `World`, sprite cache,
  textures and map position sets; registers components and resources, provides
  `add_ui`/`add_ui_sized`, world clearing, `SpriteRenderData` and live
  `apply_texture_pack` with selective cache invalidation. `texture_path_in_use`
  / `remove_sprite_if_unused` keep a cached sprite while another live entity
  still renders the same texture (shared text labels).
- **`src/ecs/adapter/render.rs`** — `get_sprites_by_layer` (8 z-layers culled by
  visible bounds), `update_object_textures` (box/rack textures by food count),
  `update_fence_textures` (fence by neighbours, pack-aware), `update_sprite_alpha`,
  `update_transform_position`.
- **`src/ecs/components.rs`** — components and resources: `Transform`,
  `SpriteComponent` (including per-sprite `anim_fps`/`anim_loop`/timer),
  `GroupComponent`, `GroupInfoResource`, `Rotation`, `ObjectTag`, `FoodStorage`,
  `ShopperTag`; `TotalFood`, `Money`, `BusyCassas`, `BasementPlaced`,
  `ShopOwned`, `FenceComponent`, `Season`, `DoorState`, `DoorProgress`.
- **`src/ecs/cursor.rs`** — `add_cursor`, `update_cursor_preview` (translucent
  object ghost, atlas frames, error markers), `clear_cursor_preview`.
- **`src/ecs/factory.rs`** — `create_sprite`: the only way to create a sprite
  (`Transform` + `SpriteComponent` from position, texture, frame, scale, alpha,
  with neutral animation defaults).
- **`src/ecs/group.rs`** — multi-cell objects: `add_group_object` (one entity per
  tile with an atlas frame offset and pack overrides; box/rack keep food-driven
  paths), `delete_group`, `find_group_at_position` (decor beats light, light
  beats carpet).
- **`src/ecs/particles.rs`** — `ParticleSystem`: short-lived place/remove
  bursts driven by `PlacementFxEvent`; `object_color` samples the dominant
  opaque pixel of the item's atlas frame (pack/mod aware), particles are
  solid 4×4 RGBA squares on `Z_WEATHER`, cached by color, capped at
  `MAX_ACTIVE_PARTICLES` for the dynamic uniform budget.
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
  `GoingToExit`), fade-in spawn, pack-provided idle/walk frames (`mark` is a
  separate id), `refresh_pack`, `is_mark`, Lua API, `update` (`npc.lua` script
  or `update_fallback` in Rust).
- **`src/npc/interactions.rs`** — `take_food`/`take_candy`, `cassa_exists`,
  `rack_exists`, `candy_exists`, `find_any_cassa`, `reroute_to_cassa`, sounds
  and textures.
- **`src/npc/movement.rs`** — `spawn_path_node`, `set_idle`/`set_walk` with
  variable-length pack frame lists, `start_path` (A* plus `exit_target`),
  `walk_toward` (step, pack fps or config interval, rotation), `walk_to_exit`.

---

## `src/scenes/` — scenes

- **`src/scenes/mod.rs`** — declares `scene_trait`, `scene_manager`,
  `menu_scene`, `cutscene`, `game`; re-exports `Scene`, `SceneManager`,
  `MenuScene`, `IntroScene`, `GameScene`.
- **`src/scenes/scene_trait.rs`** — the `Scene` trait (`on_enter`, `on_exit`,
  `update`, `sprites`, `map_size`, `camera_offset`, `night_factor`, `lights`,
  `ui_size`, custom-render hook) and `SceneAction`.
- **`src/scenes/scene_manager.rs`** — `SceneManager`: `menu`/`intro`/`game`
  registry, one ECS world, `switch_to` (stop music, clear, `on_enter`),
  `update_fps`, `clear_ecs_world`.
- **`src/scenes/menu_scene.rs`** — `MenuScene`: background map, logo,
  Play/Mods/Quit buttons, world pick/create/delete with confirmation, hover
  highlight, Space/click to start, Escape to quit. Buttons run
  Play → Settings → Mods → Quit; Settings reuses the in-game panel without the
  **Back to menu** button, Mods opens the shared scrollable mod panel.
  `ui_size()` widens in those states so 9×6 / 10×7.4 panels fit.
- **`src/scenes/cutscene/mod.rs`** — `IntroScene`: the logo-to-menu cutscene;
  timeline clock, skip on any input, `swish` on each jump, rebuilds the menu
  map/decor for the shop snapshot and owns the custom renderer.
- **`src/scenes/cutscene/timeline.rs`** — `Stage`, `Plane`, nine-stage
  `TIMELINE`, `IntroClock`, per-plane transforms and jump parameters (focus,
  zoom, blur); pure data and unit tests.
- **`src/scenes/cutscene/stars.rs`** — deterministic 296-star field with three
  parallax layers, quantized alpha, wrap only during calm stages, and the
  per-frame instance buffer.
- **`src/scenes/cutscene/renderer.rs`** — `CutsceneRenderer`: offscreen plane
  targets, instanced stars, procedural solar/continent shaders, radial-blur
  composite, menu-background snapshot and the subtle skip hint.
- **`src/scenes/game/mod.rs`** — `GameScene`: main loop — input, rent,
  bankruptcy, settings and Mods window, live texture-pack/mod-layer switching,
  food regen, per-sprite animations, day/night with mod events, shoppers,
  weather, per-season music, placement/removal particle bursts, tutorial, shop,
  hover/tooltip, autosave, level switching.
- **`src/scenes/game/camera.rs`** — `update_camera`: movement only in `move`
  mode (mouse/finger drag or arrow keys), clamped to the map's visible area.
- **`src/scenes/game/day_night.rs`** — `DayNightCycle`: `tick`, `factor`
  (0..1 dimming with dawn and dusk), `is_night`, `time_string` (`HH:MM`).
- **`src/scenes/game/hud.rs`** — `GameHud`: "Loading...", object hint,
  Food/Money counters, clock (at most once per second), item tooltip, info
  panel; text is recreated only when it changes.
- **`src/scenes/game/inventory_input.rs`** — inventory input: `E` toggles the
  grid, `←`/`→` cycle category tabs, page-arrow clicks page the tab row, grid
  clicks, move to the active slot, hotbar selection.
- **`src/scenes/game/level.rs`** — levels: `save_current_level` into
  `LevelState`, `load_level` (shop ↔ basement from cache), `rebuild_ui`,
  `save_to_disk`/`load_from_disk` (`data/worlds/world_{id}.json`), basement exit.
- **`src/scenes/game/shoppers.rs`** — `ShopperManager`: `spawn_shopper`
  (random rack/register/candy target, three regular sets or a rare Mark),
  `roll_mark`, `set_active`, `refresh_pack`, `tick` (spawn timers, limit,
  one Mark at a time, despawn, cooldown).
- **`src/scenes/game/tutorial.rs`** — `Tutorial`: tutorial panel, `open`/`close`,
  button and panel hit tests, gentle sine sway.
- **`src/scenes/game/weather_fx.rs`** — `WeatherFx`: rain and snow particles,
  `Effect`, sprite spawn/teardown, snow/rain ambient loops; re-exports the
  shared deterministic `rand01` from `core::util`.

---

## `src/ui/` — interface

- **`src/ui/mod.rs`** — declares `anim`, `components`, `fps`, `inventory`, `mods`,
  `settings`, `shop`, `system`, `text_input`, `text_renderer`, `weather`;
  re-exports components and system helpers.
- **`src/ui/components.rs`** — element descriptions: `Panel` (optional
  texture), `Button`, `Checkbox` (with `sprite_key`, `hover_scale`), `Slider`
  (track, thumb, `dragging`, `hover_scale`).
- **`src/ui/system.rs`** — UI creation and deletion: `ndc_to_ui`, `is_inside`,
  `is_clicked`, factories and destructors (`TEX_BUTTON` backgrounds),
  `slider_drag`, `update_slider_thumb`, per-widget hover scaling
  (`update_checkbox_hover`, `update_slider_hover`).
- **`src/ui/inventory.rs`** — `Inventory`: grid and paginated tabs (`TAB_PAGE_SIZE`,
  `switch_page`, `tab_page_of`, `visible_tab_count`), `items`, `switch_tab`,
  `tab_after`, grid clicks, move to the hotbar.
- **`src/ui/settings.rs`** — settings panel: VSync (`vsync_toggled`), zoom
  speed slider (`zoom_speed_changed`), music/SFX, Texture Pack and Shader
  selectors, **Mods...** button (`mods_requested`), `open`/`close`/`reset`,
  `handle_input`, `apply_saved`. `hide_menu_button()` omits the **Back to menu**
  button when the panel is opened from the main menu.
- **`src/ui/mods.rs`** — `Mods` window: styled header/count, striped rows with
  hover highlight, enable/disable checkboxes, status text (`Loaded`,
  `Disabled`, `Pending restart`, errors, version mismatch), hovered description
  in the footer, load order from checkboxes, conflict hint, `enabled_ids`,
  `refresh_statuses`. Scrolls with wheel / dragging / scrollbar when there are
  more than six mods; row clicks resolve through `mod_index_at` so scrolling
  cannot toggle the wrong mod. **Refresh** / `R` calls `refresh()` to rescan the
  folder in place. Reused by the main menu and the game's Settings panel.
- **`src/ui/shop.rs`** — `Shop`: scrollable catalogue with a scrollbar,
  `ShopItem`, `ConfirmAction`, purchase confirmation dialog, bobbing Mark
  portrait, `refresh`, `row_clicked`, `scroll_input`.
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
  an RGBA texture (outline and fill), text sprites with synthetic cache keys
  preserved on pack switches, cache keyed by text+size+colour; `add_text`,
  `set_text`, `add_text_z`. `set_text` only evicts the old sprite when no other
  entity renders the same text, so identical status lines survive a toggle.

---

## `src/input/` — input

- **`src/input/mod.rs`** — declares `camera`, `cursor`, `interact`, `platform`;
  `do_input` — the per-frame input entry: zoom, cursor, clicks and `F`, UI-zone
  exclusion, `Tab` mode switch, validity and preview, slot return.
- **`src/input/camera.rs`** — `handle_zoom`: wheel, `K`/`L`, range
  `ZOOM_MIN..ZOOM_MAX`.
- **`src/input/cursor.rs`** — `handle_mouse_movement` (NDC → world → cell,
  clamping, per-tile delay), `update_cursor_validity` (ok/error texture),
  `update_cursor_preview` with texture-pack overrides; `move` mode leaves the
  map untouched.
- **`src/input/interact.rs`** — `try_interact` (box — sell food, rack — refill,
  basement — level switch, arcade — refund 1), `cycle_mode`, `do_interact`,
  `next_mode`.
- **`src/input/platform.rs`** — `InputSource` (unified interface),
  `DesktopInput` over `winit_input_helper`, `TouchInput` (tap, drag, pinch),
  keyboard, IME and text event handling.

---

## `src/audio/`

- **`src/audio/mod.rs`** — rodio `AudioEngine`: `sounds/` cache, singleton via
  `OnceLock`/`Mutex`; output opens with a 100 ms buffer and ignores recoverable
  `BufferUnderrun` reports while logging other stream errors; `play`,
  replacing/looping `play_music` (season tracks with `music.ogg` fallback),
  `play_ambient` (snow/rain loops), `stop_music`, `set_music_enabled`,
  `set_sfx_enabled`, `init`.

---

## `src/scripts/` — Lua bridge

- **`src/scripts/mod.rs`** — declares `config`, `items`, `mods` and `npc`.
- **`src/scripts/config.rs`** — `BalanceConfig`: reads
  `assets/scripts/config.lua` through mlua, applies mod balance overrides,
  falls back to defaults on error; timings (`mark_chance` included), prices,
  capacities, economy; `publish_to_lua`.
- **`src/scripts/items.rs`** — `ItemDef`/`load`: reads
  `assets/scripts/items.lua` through mlua (size, texture, inventory icon and
  icon size, price, atlas frame, animation frames, tab, category), merges
  mod-registered items; returns `None` so `data::ItemCatalog` can use
  `BUILTIN_OBJECTS` when the file is missing or broken.
- **`src/scripts/npc.rs`** — `NpcScript`: calls `npc.lua` from Rust, packs NPC
  state into a Lua table, exposes primitives (`walk`, `take_food`, `start_path`,
  `busy`, `add_money`, `find_any_cassa`), copies `state`/`timer` back, emits
  `money_changed`, lets a mod `npc_update` handler take over, and falls back
  when the script is missing.
- **`src/scripts/mods.rs`** — player mods: `mod.json` manifests (including the
  `use` enable flag), discovery in `mods/` and `assets/mods/`, dependency
  ordering (`load_after`), sandboxed shared Lua VM (`mod.*` API), events,
  balance/item overrides, texture layers, `enabled_from` (merges `use` with the
  saved load order), `write_use_flag` / `persist_enabled` (desktop writes back
  to `mod.json`), errors/conflicts and internal tests.

---

## `src/save/`

- **`src/save/mod.rs`** — worlds: `WorldMeta`, `WorldManifest`, `list_worlds`,
  `create_world_with_name`, `unique_world_name`, `touch_world`, `delete_world`,
  `WorldSelection`/`SELECTED_WORLD`; settings `GameSettings` (including
  `texture_pack`, `shader` and the `mods` load-order array), `load_settings`,
  `save_settings`, global `SETTINGS`.

---

## `src/tests/`

- **`src/tests/mod.rs`** — declares the test modules and holds general checks:
  modes, text input, pure functions, constants (including main-menu button
  order and spacing).
- **`src/tests/helpers.rs`** — shared fixtures: ECS without a GPU, object
  placement, `FakeInput` (`InputSource` implementation), UI-to-pixel mapping.
- **`src/tests/data_integrity.rs`** — invariants of constants, the object
  catalogue, balance, day/night, sprite cache and settings, including legacy
  `texture_pack` and `shader` defaults.
- **`src/tests/placement_rules.rs`** — `can_place_at`, `add`/`remove`: cell
  categories, conflicts, economy, basement, walls.
- **`src/tests/particles_tests.rs`** — particle presets, solid-square texture,
  simulation (movement, fade, TTL), slot budget and item-coloured FX events.
- **`src/tests/shader_pack_tests.rs`** — shader discovery, WGSL loading, broken
  manifest fallback and path traversal protection.
- **`src/tests/group_tests.rs`** — multi-cell objects: layout, atlas frames,
  layers, position lookup, group deletion.
- **`src/tests/mods_tests.rs`** — mod discovery, manifest defaults, path
  sandbox, blocked Lua globals, event handling, balance/item overrides and
  texture layers without an active pack.
- **`src/tests/npc_tests.rs`** — shopper: states, movement, food and candy,
  registers, routing, fade-out, door, Mark texture recognition.
- **`src/tests/input_tests.rs`** — input through `FakeInput`: zoom, UI hit
  tests, slider, shop scrolling, world name, door fade, weather.
- **`src/tests/anim_tests.rs`** — `PanelAnim`: open, close, cancel, text
  sprites.
- **`src/tests/docs.rs`** — documentation checks: this map's completeness, live
  links and images.
- **`src/tests/texture_pack_tests.rs`** — pack discovery and naming, file
  overlay, `pack.json` object/NPC overrides, path traversal protection and
  vanilla fallback.
