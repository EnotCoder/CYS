# World and save format

There are three kinds of user data: map images and the basement map (shipped
assets), world saves (written by the game) and `data/settings.json` (written by
the settings panel). Saves and the basement map are plain text; the main map is
a PNG image.

## Where files live

| File | Desktop | Android |
|---|---|---|
| `assets/map/*.png`, `assets/basement.txt` | project / next to the binary | packed into the APK |
| `data/worlds/manifest.json` | `./data/worlds/` relative to the working directory | internal app data |
| `data/worlds/world_{id}.json` | `./data/worlds/` | internal app data |
| `data/settings.json` | `./data/settings.json` | internal app data |
| `assets/texture_pack/<id>/` | `./assets/texture_pack/<id>/`; overrides under user data | packed into the APK |
| `assets/shaders/<id>/` | `./assets/shaders/<id>/`; overrides under user data | packed into the APK |
| `assets/mods/<id>/` | `./assets/mods/<id>/`; overrides under user data | packed into the APK |

`core::asset` hides the difference: `load_bytes` reads project assets (on
Android through `AAssetManager`, stripping the `assets/` prefix), while
`save_data` / `load_data` / `delete_data` always target the user data directory
(`data/` on desktop, `internal_data/data/` on Android).

The whole `data/` directory is in `.gitignore`: worlds, the manifest and
`settings.json` are created locally and never land in a commit. Player content
is separate: the manifests under `assets/texture_pack/` and `assets/shaders/`
are committed, as are `assets/mods/*/mod.json` with the shipped `use` flag.
On an installed desktop build, toggling a mod writes an override under the
user data directory instead of changing these files. See
[texture-packs.md](texture-packs.md), [shaders.md](shaders.md) and
[mods.md](mods.md).

## Map files

The main map is a single PNG image, not a text grid. It is 16 px per cell, so
the shipped 848×448 image is 53×28 cells. Row `j` maps to world
`y = WORLD_OFFSET_Y - j`; column `i` maps to `x = WORLD_OFFSET_X + i`
(`WORLD_OFFSET_X = -26`, `WORLD_OFFSET_Y = 14`). The first image row is the top
of the map.

| File | Season |
|---|---|
| `assets/map/map.png` | Summer (default) |
| `assets/map/spring_map.png` | Spring |
| `assets/map/autumn_map.png` | Autumn |
| `assets/map/winter_map.png` | Winter |

All four have the same cell categories; only the grass pixels differ. The
image is rendered as one virtual-sized sprite (`...png@53x28`) at `Z_MAP`, and
`Season::map_texture()` / `set_map_season()` swap the texture when the weather
changes. Only the door has separate tile sprites.

`image_map.rs` derives gameplay categories by comparing each 16×16 tile with
the frames of `grass.png`, `stone_road.png`, `floor.png`, `wall.png` and
`water_shop_wall.png` (all flips included). Overlapping layers are resolved by
priority `water_wall > wall > floor > road > grass`. The result is reduced to
four tokens so the existing placement and pathfinding logic keeps working:

| Token | Source categories | Meaning |
|---|---|---|
| `.` | grass, road | outdoor, walkable, plantable |
| `0` | floor | walkable, items can be placed |
| `W` | wall, water wall | solid, wall decor only |
| `E` | door override | animated shop door, walkable |

The door is not part of the image: `DOOR_X_MIN..=DOOR_X_MAX` ×
`DOOR_Y_MIN..=DOOR_Y_MAX` (2×2) is forced to `E` after classification. Its
sprites form a 2×2 atlas with `Z_DOOR = Z_DECOR` and one frame per cell. Their
width is `DOOR_TILE_X_SCALE` (slightly above 1) while the height stays exactly
one cell (`DOOR_TILE_Y_SCALE = 1.0`): the separately rendered door quads overlap
the baked map background on the sides, so no 1-pixel seam appears when the
camera has a fractional offset. Baked walls are static —
`recompute_wall_token` only touches the hand-authored wall tokens of
`basement.txt`.

Valid cells for walking are computed by `load_walkable_cells` from the parsed
image; `W` is not walkable, `.`, `0` and `E` are. The
`image_map_classifies_all_tiles_and_overrides_door` test guards the 53×28 size,
the category counts and the door override; `every_map_token_resolves_to_an_in_bounds_atlas_frame`
guards the token-to-texture mapping.

### Basement map

`assets/basement.txt` is still a whitespace-separated token grid read by
`load_basement_to_ecs`. It uses the full historical token set
(`= - h d ^ & W S [ ] : ; o % q p i / | ( { ) }`, grass and floor variants),
one sprite per cell, with generated walls recomputed after placement changes.

## World manifest

`data/worlds/manifest.json` is the Minecraft-style index. `next_id` only grows, ids
are never reused, and the list is sorted by `updated` when displayed.

```json
{
  "next_id": 3,
  "worlds": [
    {
      "id": 0,
      "name": "Shop",
      "created": 1760000000,
      "updated": 1760003600
    }
  ]
}
```

Names are unique: `create_world_with_name` appends `(1)`, `(2)`… on collision,
and an empty name becomes `World`.

## World save

`data/worlds/world_{id}.json` is written by `GameScene::save_to_disk`, on exit, on
`Ctrl+S` and when leaving to the menu. It is a full snapshot, not a diff.

```json
{
  "levels": {
    "0": {
      "map_grid": [[".", "."], [".", "."]],
      "original_tokens": [],
      "objects": []
    }
  },
  "current_level": 0,
  "money": 150,
  "total_food": 0,
  "slots": ["box", "sign", "rack", "table", "cassa"],
  "act_slot": 0,
  "mode": 0,
  "camera_offset_x": 0.0,
  "camera_offset_y": 0.0,
  "map_size": 0.6,
  "active": true,
  "basement_placed": false,
  "busy_cassas": [],
  "shop_owned": []
}
```

| Field | Meaning |
|---|---|
| `levels` | level id → snapshot (`0` shop, `1` basement) |
| `map_grid` | current per-cell tokens, so rebuilt walls/floors survive |
| `original_tokens` | `[x, y, token]` of cells changed from the source map |
| `objects` | placed objects: name, cell, food count, max food, carpet flag |
| `current_level` | which level to load first |
| `money`, `total_food` | economy resources |
| `slots`, `act_slot`, `mode` | hotbar contents, active slot, current mode |
| `camera_offset_x/y`, `map_size` | camera position and zoom |
| `active` | shop open/closed state |
| `basement_placed` | whether the basement entrance was built |
| `busy_cassas` | registers reserved by shoppers (restored as free) |
| `shop_owned` | purchased shop items (`#[serde(default)]`, older saves load fine) |

A corrupt or missing world file makes `load_from_disk` return `false`; the game
falls back to a fresh level instead of crashing. Unknown JSON fields are ignored
by `serde`, and missing optional fields fall back to defaults where declared.

## Settings

`data/settings.json` is separate from worlds: it applies to the application, not to a
save.

```json
{
  "vsync": true,
  "zoom_speed": 0.08955699,
  "music": true,
  "sfx": true,
  "texture_pack": "neon",
  "shader": "crt",
  "mods": ["double_money", "big_shop"]
}
```

| Field | Default | Meaning |
|---|---|---|
| `vsync` | `true` | present mode `Fifo` vs `Mailbox` |
| `zoom_speed` | `0.1` | wheel/pinch zoom step |
| `music` | `true` | background music |
| `sfx` | `true` | sound effects |
| `texture_pack` | `null` | active pack id; `null` means Vanilla |
| `shader` | `null` | active shader id; `null` means the built-in WGSL |
| `mods` | `[]` | enabled mod ids **in load order**; later mods win |

The file is rewritten on every settings change and on exit. A missing or
unparsable file silently falls back to `GameSettings::default()`. Older files
without `texture_pack`, `shader` or `mods` load as Vanilla / no mods because the
fields have `#[serde(default)]`. An id that no longer has a pack or mod folder is
ignored; a shader that fails to load or compile keeps vanilla pipelines and
reports the reason in Settings. Mods toggled in the menu or in
**Settings → Mods...** are saved here and in each mod's `mod.json` as `use`
(desktop only). When `use` is present it overrides this array for that mod;
mods without the field follow the saved list. See [mods.md](mods.md) for load
order and when changes take effect.

### Mod manifests

Each mod folder has a `mod.json`. The `use` field is a boolean enable flag;
the desktop Mods panel rewrites it when a checkbox is toggled:

```json
{
  "name": "Double Money",
  "version": "1.0.0",
  "entry": "main.lua",
  "use": false
}
```

On Android the bundled manifests under `assets/mods/` are read-only, so the flag
cannot be written there and the choice stays in `settings.json`. A mod without
`use` is enabled only if its id appears in the `mods` array. See
[mods.md](mods.md) for the full field list.

## Compatibility notes

- Adding a field with `#[serde(default)]` keeps old saves loadable; adding a
  required field breaks them. Prefer defaults.
- `map_grid` stores tokens, not editor objects: if a token is removed from
  `token_to_texture`, old saves render it as default floor.
- `data/worlds/manifest.json` is the source of truth for the world list. Deleting a
  `world_{id}.json` manually leaves a menu entry that loads as a new world.
- `next_id` is never derived from existing files, so restoring an old manifest
  can produce duplicate ids. Keep the manifest with the world files.
