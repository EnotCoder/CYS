# Texture packs

A texture pack replaces existing art and animation data. It cannot add new
objects, NPCs or gameplay logic; it only overrides what the game already draws.
Packs can also change an object's atlas layout, add or remove object animation,
and replace shopper idle/walk frames. To add items or change balance, use
[mods](mods.md) instead — a mod's `textures/` folder layers above the pack.

## Pack locations

On desktop, the writable root is `texture_pack/` next to the working directory;
`assets/texture_pack/` is the shipped copy used by installed builds. On Android
only the APK copy under `assets/texture_pack/` is scanned. Both roots use the
same pack ids, so a pack can be shipped for both platforms by mirroring its
folder.

Each immediate subdirectory is a pack **id** when it contains either a
`textures/` directory or a `pack.json` file. Dots at the start of a name are
ignored. Packs are sorted by id, and `Vanilla` is always the first entry in the
Settings list. The selected id is stored globally in `data/settings.json`, not
in a world save.

## Layout

```text
texture_pack/
  neon/
    pack.json                 # optional
    textures/                 # mirror of assets/tex/
      map/grass.png
      decor/regular/box/box_0.png
      characters/bob/bob.png
      ui/icon_slots/icon_slots_object/regular/box.png
  cozy/
    ...
```

The `textures/` tree mirrors `assets/tex/` without the prefix: a replacement for
`assets/tex/map/grass.png` goes to `textures/map/grass.png`. A pack containing
only `textures/` already works as a simple file overlay; `pack.json` is only
needed for aliases and animation overrides.

## `pack.json`

All fields are optional. Paths inside the manifest are relative to the pack
root, not to `textures/`.

```json
{
  "name": "Neon Pack",
  "description": "Neon shop",
  "author": "someone",
  "version": "1.0.0",
  "textures": {
    "assets/tex/ui/button/main.png": "textures/ui/neon_button.png"
  },
  "objects": {
    "box": {
      "texture": "textures/decor/box_neon.png",
      "frame": [0, 0],
      "atlas": [2, 2],
      "animated": true,
      "frames": [
        "textures/decor/box_neon_0.png",
        "textures/decor/box_neon_1.png"
      ],
      "fps": 4.0,
      "loop": true
    },
    "arcade_machine": {
      "frames": ["textures/arcade_0.png", "textures/arcade_1.png"],
      "fps": 6.0
    }
  },
  "npcs": {
    "bob": {
      "idle": ["textures/characters/bob_idle_0.png"],
      "walk": [
        "textures/characters/bob_walk_0.png",
        "textures/characters/bob_walk_1.png"
      ],
      "fps": 6.0
    }
  }
}
```

| Field | Meaning |
|---|---|
| `name` | Label shown in Settings; falls back to the pack id |
| `description`, `author`, `version` | Parsed metadata, not shown in the UI yet |
| `textures` | Explicit map: logical asset path → file inside the pack; useful for paths outside `assets/tex/` |
| `objects.<name>` | Override for a catalogue object (`box`, `rack`, `arcade_machine`, …) |
| `objects.*.texture` | Replacement base texture file |
| `objects.*.frame` | Starting atlas frame `[col, row]` |
| `objects.*.atlas` | Atlas size `[cols, rows]`; values below 1 are clamped to 1 |
| `objects.*.animated` | `false` disables animation completely |
| `objects.*.frames` | List of frame files; any readable entry enables a frame sequence |
| `objects.*.fps` | Frames per second, default `1.0`, minimum `0.01` |
| `objects.*.loop` | Whether the sequence restarts at the end, default `true` |
| `npcs.<id>` | `bob`, `player` or `sasha` |
| `npcs.*.idle`, `npcs.*.walk` | Lists of frame files; omitted lists stay vanilla |
| `npcs.*.fps` | Overrides both idle frame timing and the walk animation interval |

An absolute path, a `..` component, a Windows separator or an empty path is
rejected. A bad manifest logs an error and the pack still works as a plain file
overlay.

## Resolution order

For a logical texture path the active pack is consulted in this order:

1. an enabled [mod](mods.md)'s `textures/<path after assets/tex/>`, from the
   last loaded mod to the first (mods work even with no pack selected);
2. `textures/<path after assets/tex/>` inside the pack;
3. an exact key from the manifest's `textures` map;
4. the vanilla `assets/tex/...` path;
5. `assets/tex/dev_tools/null.png` if nothing can be loaded.

The mirror is checked first, so it wins over an explicit `textures` entry for
the same path. Mod layers have the highest priority and are applied live when
mods are toggled. If a file is present but cannot be decoded, the loader retries
the next layer before showing `null.png`.

A virtual size suffix such as `@2x1` is not part of a file name. It is stripped
for lookup and still controls how the sprite is stretched, because the sprite
cache key keeps the original path. This matters for objects such as
`big_lamp.png@1x2`.

## Animation

Every sprite carries its own `fps`, `loop` flag and timer. The default is one
frame per second with looping, which matches the old global timer. A non-looping
sequence stops on its last frame. Only readable frame files are used; if all of
them fail, the vanilla frame list remains.

Shopper ids are inferred from the vanilla idle path: it contains `bob`,
`player` or `mark`, otherwise the shopper is treated as `sasha`. The rare Mark
shopper uses the `mark` id and can be overridden the same way. A pack with a
single idle frame does not animate while waiting; longer lists cycle at the
pack's `fps`.
Walk frames cycle at `fps` instead of `walk_anim_interval` from
[scripts.md](scripts.md). New shoppers use the active pack at spawn, and
existing shoppers are refreshed when the pack changes. The shop's Mark portrait
uses `assets/tex/characters/mark_salesman/mark_full_face.png`, resolves like any
other UI texture and keeps its vanilla sine bob.

`box` and `rack` are exceptions: their texture is chosen by the amount of food
they contain. Their logical vanilla paths and food-driven state are preserved,
and a pack can still replace the underlying files, but `animated`, `frames`,
`fps` and `loop` do not drive them.

## Selecting a pack

Open **Settings** and use the `<` / `>` buttons on the **Texture Pack** row. The
list is rescanned each time Settings opens, so packs can be added while the
desktop game is running. Changing the selection:

- sets the active pack immediately, without reloading the level;
- clears asset-backed texture and sprite caches so map, decor, UI, cursor, door
  and fences are redrawn from the new pack;
- keeps generated text and weather sprites, whose paths are not under `assets/`;
- reapplies object overrides and refreshes living shoppers;
- writes `texture_pack` to `data/settings.json` (see
  [world-format.md](world-format.md)).

An unknown or missing id falls back to Vanilla at startup. Camera, placed
objects, money and the current level are untouched by a pack switch.

## Adding a pack

1. Create `texture_pack/<id>/textures/` on desktop, or
   `assets/texture_pack/<id>/textures/` before building for Android.
2. Put replacement PNG files under the mirrored paths.
3. Add an optional `pack.json` for aliases, object overrides or NPC frames.
4. Open Settings, cycle to the pack and check map, objects, UI and shoppers.
5. Run `cargo test`; the pack tests cover path safety and vanilla fallback.

`pack.json` files under `texture_pack/` are explicitly unignored by
`.gitignore`, so a pack can be committed if desired. See
[assets.md](assets.md) for the vanilla texture tree and file conventions.

## Limitations

- No new objects, NPCs, sounds or mechanics; only existing textures and
  animation data are overridden.
- Frame sequences are separate files inside the pack, not coordinate lists
  inside an atlas.
- Gameplay timings (door, rent, day/night) are unchanged; only animation `fps`
  is pack-controlled, and not for `box`/`rack`.
- Android packs must be included in the APK at build time.
- Only the pack `name` is displayed; the other metadata fields are stored for
  future UI work.
