# ECS

[Русский](../ru/ecs.md) · English

CYS uses [`specs`](https://docs.rs/specs) 0.18, but only as a storage: there is
no `Dispatcher` and no schedule. Gameplay code calls plain functions from
`GameScene::update`; the ECS just holds entities and lets render code query
them. `EcsAdapter` (`src/ecs/adapter/mod.rs`) is the single façade over
`specs::World`.

## What the adapter owns

| Field | Purpose |
|---|---|
| `world` | components and resources |
| `sprite_cache` | GPU sprites keyed by `sprite_cache_key` |
| `texture_cache` | decoded textures keyed by base path (without `@WxH`) |
| map position sets | entity lookup by cell (`map_entities`) |

`EcsAdapter::new()` registers every component and resource once. Adding a new
component means registering it there too, otherwise `world.write_storage::<T>()`
panics at runtime.

## Creating entities

There are exactly three sanctioned entry points; do not assemble
`Transform` + `SpriteComponent` by hand:

| Call | What it creates |
|---|---|
| `ecs::factory::create_sprite` | one sprite entity: `Transform`, `SpriteComponent`, `Rotation` |
| `EcsAdapter::add_group_object` | N entities for a multi-cell object, each with a frame offset |
| `EcsAdapter::add_ui` / `add_ui_sized` | UI sprites in UI space (`map_size = 1.0`) |

A sprite is a quad with a texture, an atlas frame (`[col, row]`), an atlas size
(`[cols, rows]`), a scale and an alpha. `Sprite::split_sized_path` allows
`path.png@2x2` to stretch one tile over a 2×2 footprint.

## Components and resources

| Type | Kind | Meaning |
|---|---|---|
| `Transform` | component | `position`, `scale`, `rotation` |
| `SpriteComponent` | component | texture path, frame, atlas, alpha |
| `Rotation` | component | rotation vector |
| `ObjectTag` | component | object name (`rack`, `cassa`, …) for gameplay lookup |
| `FoodStorage` | component | `food_count`, `max_food` |
| `GroupComponent` | component | group id and cell offset inside a multi-cell object |
| `FenceComponent` | component | fence orientation |
| `ShopperTag` | component | marker used by the door to open |
| `GroupInfoResource` | resource | group id → entities, size, name |
| `Money`, `TotalFood` | resource | economy counters |
| `BusyCassas` | resource | registers currently reserved by shoppers |
| `BasementPlaced` | resource | whether the basement entrance exists |
| `ShopOwned` | resource | purchased shop items |
| `Season` | resource | `Summer`, `Spring`, `Autumn`, `Winter` |
| `DoorState`, `DoorProgress` | resource | door animation state and frame |

## Render data

`get_sprites_by_layer(bounds)` walks the storages once per layer and returns
eight `Vec<SpriteRenderData>`: map, carpet, light, decor, NPC, weather, cursor,
UI. A `SpriteRenderData` is a flat struct (position, rotation, texture path,
frame, atlas, scale, alpha) — no `specs` types, no borrows, safe to pass to the
renderer.

Culling happens here: sprites outside `bounds` are dropped, and sprites with
zero alpha are dropped too. On top of that, `update_object_textures` swaps
box/rack frames by food count, and `update_fence_textures` picks a fence frame
from its neighbours.

## Multi-cell objects

`add_group_object` splits a W×H object into W·H entities, one per cell, each
with `GroupComponent { group, offset }` and a frame shifted by the offset. This
keeps the renderer dumb: every tile is still a normal sprite.

`delete_group` removes all entities of a group. `find_group_at_position`
returns the object under the cursor with a fixed priority: decor beats light,
light beats carpet. That priority is the reason a lamp can stand on a carpet and
a rack on a floor.

## Cursor preview

The cursor is a normal sprite on `Z_CURSOR`. `update_cursor_preview` builds a
translucent ghost of the object being placed, using its real atlas frames, and
switches to an error marker when placement is invalid. `clear_cursor_preview`
removes it. Because the ghost is an entity, it goes through the same render path
as everything else.

## Placement rules

Rules are split in two:

- `ecs::placement::can_place_at` — pure predicate: cell category, category
  conflicts, wall/floor/grass requirements;
- `data::placement::{add, remove}` — side effects: money, refunds, wall
  recomputation, `FoodStorage`, `BasementPlaced`, sounds.

Keeping the predicate pure makes it cheap to test: `src/tests/placement_rules.rs`
builds an ECS without a GPU and checks both directions.

## Why no systems

A `specs` dispatcher would add a parallel scheduling model for no benefit: the
game has one scene active at a time and a strictly sequential tick. Plain
functions are easier to debug, easier to test, and let `GameScene` control the
order (input → economy → NPCs → day/night → render data).

The trade-off is discipline: every new component must be registered, and every
new layer must be wired into `Scene::sprites` and `core::render`. The
[source map](source-map.md) and tests are there to catch omissions.
