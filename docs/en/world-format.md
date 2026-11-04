# World and save format

[Русский](../ru/world-format.md) · English

There are three kinds of user data: map files (shipped assets), world saves
(written by the game) and `settings.json` (written by the settings panel). All
of them are plain text, so a broken save can be inspected and repaired by hand.

## Where files live

| File | Desktop | Android |
|---|---|---|
| `assets/map.txt`, `assets/basement.txt` | project / next to the binary | packed into the APK |
| `worlds/manifest.json` | `./worlds/` relative to the working directory | internal app data |
| `worlds/world_{id}.json` | `./worlds/` | internal app data |
| `settings.json` | `./settings.json` | internal app data |

`core::asset` hides the difference: `load_bytes` reads project assets (on
Android through `AAssetManager`, stripping the `assets/` prefix), while
`save_data` / `load_data` / `delete_data` always target the user data directory.

`worlds/` and `*.json` are in `.gitignore`; `settings.json` is tracked as a
repository default but is still overwritten at runtime.

## Map files

A map is a whitespace-separated grid, one token per cell. Row `j` maps to world
`y = WORLD_OFFSET_Y - j`; column `i` maps to `x = WORLD_OFFSET_X + i`
(`WORLD_OFFSET_X = -26`, `WORLD_OFFSET_Y = 14`). So the first row is the top of
the map and `y` grows downwards in file order.

```
grass      . @ * m f ~ l H K 1 2 3 4 5 6
floor      P Q Z X , _ A D 0
wall       = - h d ^ & W S [ ] : ; o % q p i
grass edge / | ( { ) }
door       E
```

- Grass tokens select an atlas frame from `assets/tex/map/grass.png`
  (`[col, row]` inside a 4×6 atlas); the season changes the texture file, not
  the frame.
- Floor tokens come from `floor.png` (3×3 atlas), wall tokens from `wall.png`
  (5×5 atlas). Walls are recomputed when objects are added or removed
  (`recompute_wall_token`), so hand-drawn wall corners are only the starting
  state.
- `E` is the animated shop door: a 2×2 atlas, one frame per cell, with
  `Z_DOOR = Z_DECOR`.
- Any unknown token silently becomes a default floor tile. The
  `every_map_token_resolves_to_an_in_bounds_atlas_frame` test guards the known
  set.

Valid cells for walking are computed by `load_walkable_cells` from the same
tokens; a wall token is not walkable, floor and door are.

## World manifest

`worlds/manifest.json` is the Minecraft-style index. `next_id` only grows, ids
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
and an empty name becomes `Мир`.

## World save

`worlds/world_{id}.json` is written by `GameScene::save_to_disk`, on exit, on
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

`settings.json` is separate from worlds: it applies to the application, not to a
save.

```json
{
  "vsync": true,
  "zoom_speed": 0.08955699,
  "music": true,
  "sfx": true
}
```

| Field | Default | Meaning |
|---|---|---|
| `vsync` | `true` | present mode `Fifo` vs `Mailbox` |
| `zoom_speed` | `0.1` | wheel/pinch zoom step |
| `music` | `true` | background music |
| `sfx` | `true` | sound effects |

The file is rewritten on every settings change and on exit. A missing or
unparsable file silently falls back to `GameSettings::default()`.

## Compatibility notes

- Adding a field with `#[serde(default)]` keeps old saves loadable; adding a
  required field breaks them. Prefer defaults.
- `map_grid` stores tokens, not editor objects: if a token is removed from
  `token_to_texture`, old saves render it as default floor.
- `worlds/manifest.json` is the source of truth for the world list. Deleting a
  `world_{id}.json` manually leaves a menu entry that loads as a new world.
- `next_id` is never derived from existing files, so restoring an old manifest
  can produce duplicate ids. Keep the manifest with the world files.
