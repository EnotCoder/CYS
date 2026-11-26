# Testing

Tests live in `src/tests/` and are compiled into the library under
`#[cfg(test)]`. They are fast and do not need a GPU, a window or a sound
device: rendering and input are represented by plain data and a fake input
source.

```sh
cargo test
cargo test docs
cargo test -- --nocapture
```

`cargo test docs` runs only the documentation checks. A single test can be
selected by name, for example `cargo test door`.

## Suites

| File | Covers |
|---|---|
| `src/tests/mod.rs` | mode cycle, text input, pure helpers, UI scale, map tokens, catalogue |
| `src/tests/helpers.rs` | shared fixtures: `new_ecs`, placement helpers, `FakeInput`, UI pixel mapping |
| `src/tests/data_integrity.rs` | constants, `ALL_OBJECTS`, balance config, day/night, sprite cache, settings |
| `src/tests/placement_rules.rs` | `can_place_at`, `add`, `remove`: categories, conflicts, refunds, walls, basement |
| `src/tests/particles_tests.rs` | place/remove presets, solid-square texture, item-coloured events, movement/fade/TTL, `MAX_ACTIVE_PARTICLES` |
| `src/tests/group_tests.rs` | multi-cell layout, atlas frames, layers, `find_group_at_position`, deletion |
| `src/tests/npc_tests.rs` | shopper states, movement, food/candy, registers, routing, fade, door |
| `src/tests/input_tests.rs` | zoom, UI hit tests, slider, shop scrolling, world name, door fade, weather |
| `src/tests/anim_tests.rs` | `PanelAnim` open/close/cancel and text sprites |
| `src/tests/mods_tests.rs` | mod discovery, manifest defaults and `use` parsing, path sandbox, blocked Lua globals, events, balance/item overrides, texture layers, `enabled_from`, `write_use_flag`, bundled manifests |
| `src/ui/mods.rs` (unit test) | scroll offset and scrollbar thumb math; a scrolled row resolves to its own mod id; shared status text sprite is kept while another mod row still uses it |
| `src/tests/texture_pack_tests.rs` | file overlay, `pack.json` overrides, path safety, vanilla fallback |
| `src/tests/shader_pack_tests.rs` | shader discovery, WGSL loading, broken manifest fallback, path traversal protection |
| `src/tests/docs.rs` | source map completeness and live links |

The suite currently has 144 tests. They are ordinary `#[test]` functions, not
`specs` systems, so a failure points at a function name, not at a schedule.

## Writing a test without a GPU

`helpers::new_ecs` builds an `EcsAdapter` with registered components but no
device or queue. Anything that only touches the ECS — placement, groups,
economy, pathfinding, NPC state — can be tested that way. Use
`create_sprite`/`add_group_object` exactly like gameplay code.

`FakeInput` implements `InputSource` and lets a test say "the left button was
pressed this frame, the cursor is at NDC (x, y), the wheel moved by 3". That is
how zoom, sliders, shop scrolling and UI hit tests are covered.

Assets are read through `core::asset`, so tests that check icons or maps use
the real files from the repository. `image_map::cached_map()` parses the shipped
PNG map once and is used by both gameplay code and tests; the basement map is
read from `assets/basement.txt`.

Rules of thumb:

- no sleeps and no real time: pass `dt` explicitly;
- no randomness in assertions: `rand01` is deterministic, and NPC tests use
  fixed seeds or fixed positions;
- no GPU calls: if a function needs `device`/`queue`, test the pure part around
  it instead;
- assert messages should explain the failure in Russian, matching the codebase.

## Documentation tests

`src/tests/docs.rs` is part of `cargo test` and enforces the docs contract:

1. every `src/**/*.rs` path appears in `docs/source-map.md`, and every
   `src/...` path mentioned there exists;
2. every relative Markdown link and image resolves to an existing file.

This is why a new module must be added to the source map in the same change.

## Manual smoke test

Before a release, run the game and check:

- new world → tutorial → place box/rack/register → open shop; each build and
  each `delete` throws small pixel-art squares in that item's own colour (brown
  for a rack, etc.), even when several objects are placed quickly;
- a shopper walks in, buys, pays, leaves through the door;
- `Esc` pauses rent and day/night;
- switching two texture packs in Settings updates map, objects, UI and shoppers
  without reloading the level;
- **Settings** button in the main menu (after Play) opens the in-game settings
  without the **Back to menu** button; changing VSync, sound, pack, shader or
  zoom writes `data/settings.json`, and `Escape` returns to the menu;
- **Mods** button in the main menu opens the panel; toggling `double_money` /
  `big_shop` shows `Pending restart`, and re-entering the world applies balance
  and item changes; disabling one mod and enabling the other keeps both status
  lines visible (no `null` texture on the disabled row);
- with more than six mods, wheel / drag scrolling reaches the last row and the
  scrollbar thumb follows; clicking a checkbox after scrolling toggles the mod
  on that row, not the one at the top of the list;
- adding or removing a mod folder while the panel is open and pressing **Refresh**
  (or `R`) updates the list and counter without closing it, keeping other
  checkboxes as they were;
- **Settings → Mods...** shows the same state in-game; toggling updates `use`
  in `assets/mods/<id>/mod.json` in a source checkout (or the user-data
  override on an installed build);
- shop purchase survives `Ctrl+S` and a reload from the menu;
- Android: Back closes panels, pinch zooms, one-finger drag moves the camera
  only in `move` mode.

## What is not covered

- GPU output (no golden-image tests);
- audio playback;
- Android JNI and `EditText` behaviour;
- real time-based interactions (rent after 60 s is tested through values, not
  by waiting).

Those are checked manually; keep the manual checklist short and repeatable.
