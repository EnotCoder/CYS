# Contributing

## Before you change code

- Read [architecture.md](architecture.md) — the 12-folder layout is intentional.
- For anything visual, read [rendering.md](rendering.md); for item logic, read
  [ecs.md](ecs.md); for balance, [scripts.md](scripts.md).
- Run `cargo test` once before your first change, so you know the baseline.

## Module rules

- `src/` keeps exactly 12 top-level folders. Do not add a 13th without a strong
  reason; most code belongs in an existing module.
- A new file is declared in the parent `mod.rs` and, if it is part of the public
  surface, re-exported there.
- `lib.rs` declares modules with `#[path = "../<module>/mod.rs"]`; keep that
  pattern.
- A type's `impl` blocks stay in one file. Trait impls cannot be split across
  submodules, so plan for it.
- When moving files: move first, then update every `use` path, then build.

## Code style

- No comments on trivial code. Comments explain *why*, not *what*.
- Comments and assertion messages are in Russian; identifiers and code are in
  English.
- Group imports: `use crate::{A, data::B};`.
- Prefer `replaceAll` when renaming a symbol rather than many small edits.
- Keep functions short enough to name their side effects. If a function both
  mutates the ECS and draws UI, split it.
- Prefix SPDX headers on new files:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder
```

## ECS rules

- Create entities only through `create_sprite`, `add_group_object` or
  `add_ui`/`add_ui_sized`.
- Register every new component and resource in `EcsAdapter::new`.
- Do not query `specs` from render code; go through
  `get_sprites_by_layer`.
- Keep placement predicates pure (`can_place_at`) and side effects in
  `data::placement`.
- Multi-cell objects must go through groups, not hand-built entity lists.

## Rendering rules

- A new layer needs: a `Z_*` constant, a sprite list in `Scene::sprites`, a
  parameter in `core::render`, and a slot in the transparent or opaque pass.
- Anything with alpha belongs to the transparent pass; anything below `Z_MAP`
  must be opaque.
- Any struct shared with WGSL must match the `std430` layout, including
  padding (see `LightData`, 64 bytes).
- Pixel art uses `FilterMode::Nearest`; do not introduce `Linear` for sprites.

## Assets rules

- Paths always start with `assets/` and are read through `core::asset`.
- Object icons go to `assets/tex/ui/icon_slots/icon_slots_object/<category>/`.
- New textures follow the existing naming: `snake_case`, frames as `_0`, `_1`,
  or `_walk_1`, `_walk_2`.
- Run `cargo test` after adding an object: icon and catalogue tests will tell
  you what is missing.

## Documentation rules

The docs are part of the build. When you change code, check whether one of
these pages is now wrong:

| Change | Update |
|---|---|
| new/removed/renamed `.rs` file | `docs/source-map.md` |
| new module or frame step | `docs/architecture.md` |
| new layer, pipeline, uniform | `docs/rendering.md` |
| new component, resource, group rule | `docs/ecs.md` |
| balance, shopper, season, economy | `docs/gameplay.md` + `scripts.md` |
| save field, map token, settings field | `docs/world-format.md` |
| new asset directory or naming rule | `docs/assets.md` |
| build command, target, profile | `docs/build.md` |
| new test suite or fixture | `docs/testing.md` |

Documentation is written in English. After editing, run:

```sh
cargo test docs
```

## PR checklist

- [ ] `cargo test` passes;
- [ ] no `target/` or local `data/` changes in the diff;
- [ ] new files have SPDX headers;
- [ ] new paths start with `assets/`;
- [ ] docs updated per the table above;
- [ ] manual smoke test done if the change is visible in game;
- [ ] no debug prints left behind;
- [ ] commit message describes the behaviour, not the files (`add buy
  confirmation dialog`, not `update shop.rs`).

## Licensing

The project is GPL-3.0. By contributing you agree to license your changes under
the same terms. See [LICENSE](../LICENSE).
