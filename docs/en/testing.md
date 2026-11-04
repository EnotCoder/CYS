# Testing

[Русский](../ru/testing.md) · English

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
| `src/tests/group_tests.rs` | multi-cell layout, atlas frames, layers, `find_group_at_position`, deletion |
| `src/tests/npc_tests.rs` | shopper states, movement, food/candy, registers, routing, fade, door |
| `src/tests/input_tests.rs` | zoom, UI hit tests, slider, shop scrolling, world name, door fade, weather |
| `src/tests/anim_tests.rs` | `PanelAnim` open/close/cancel and text sprites |
| `src/tests/docs.rs` | RU/EN pairings, source map completeness, identical code blocks, live links |

Total is around 90 tests. They are ordinary `#[test]` functions, not
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
the real files from the repository. `include_str!("../../assets/map.txt")` is
used where a compile-time dependency is acceptable.

Rules of thumb:

- no sleeps and no real time: pass `dt` explicitly;
- no randomness in assertions: `rand01` is deterministic, and NPC tests use
  fixed seeds or fixed positions;
- no GPU calls: if a function needs `device`/`queue`, test the pure part around
  it instead;
- assert messages should explain the failure in Russian, matching the codebase.

## Documentation tests

`src/tests/docs.rs` is part of `cargo test` and enforces the docs contract:

1. every `docs/ru/*.md` has an `docs/en/*.md` twin and vice versa;
2. every `src/**/*.rs` path appears in both `source-map.md` files, and every
   `src/...` path mentioned there exists;
3. fenced code blocks are identical between the RU and EN versions of a page;
4. every relative Markdown link and image resolves to an existing file.

This is why a new module must be added to the source map in the same change,
and why code examples in translations are copied verbatim rather than
reformatted.

## Manual smoke test

Before a release, run the game and check:

- new world → tutorial → place box/rack/register → open shop;
- a shopper walks in, buys, pays, leaves through the door;
- `Esc` pauses rent and day/night;
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
