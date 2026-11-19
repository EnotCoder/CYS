# Player mods

Mods extend the game with Lua logic and optional assets. Unlike
[texture packs](texture-packs.md) and [shaders](shaders.md), a mod changes
behaviour: balance, the item catalogue, shopper logic and game events. A mod
that is disabled, missing or broken never affects the game — vanilla values
take over.

## Locations

| Platform | Root |
|---|---|
| Desktop | `mods/<id>/` next to the working directory |
| Android | `assets/mods/<id>/`, bundled into the APK at build time |

Each immediate subdirectory with a `mod.json` is a mod; its folder name is the
mod **id**. The two bundled examples ship in both locations: `double_money`
(shoppers pay more) and `big_shop` (extra starting money plus a neon box item).
They are disabled by default. On desktop the folder can be extended while the
game is running — the list is rescanned when the **Mods** window opens. Android
mods require an APK rebuild.

## Layout

```text
mods/
  double_money/
    mod.json
    main.lua
  big_shop/
    mod.json
    main.lua
    textures/
      decor/neon_box.png
```

`mod.json` is required; `main.lua` is the default entry point.

```json
{
  "name": "Double Money",
  "description": "Shoppers pay twice as much",
  "author": "CYS",
  "version": "1.0.0",
  "entry": "main.lua",
  "load_after": [],
  "game_version": "0.1.3",
  "use": false
}
```

| Field | Meaning |
|---|---|
| `name`, `description`, `author`, `version` | metadata shown in the Mods window |
| `entry` | main Lua file, defaults to `main.lua` |
| `load_after` | mod ids that must load before this one |
| `game_version` | game version the mod targets; a mismatch is flagged in the UI |
| `use` | `true` to enable the mod on startup, `false` to keep it off; omitted means "follow `settings.json`" |

All fields have defaults, so a minimal `{"name":"My Mod"}` is valid. Paths
inside a mod are relative to its folder; `..`, absolute paths and backslashes
are rejected.

`use` is authoritative when present: on startup it overrides the `mods` array
in [settings.json](#enabling-mods) for that mod, while mods without the field
still follow the saved list. The load order from settings is preserved, and
`use: true` mods missing from settings are appended in discovery order. When a
checkbox is toggled in the Mods window, the game writes `use` back to every
`mod.json` on desktop. Android assets are read-only, so there the choice is
kept only in settings.

## Lua API

Mods run in a sandbox. The unsafe globals `io`, `os`, `require`, `load`,
`loadstring`, `package`, `debug`, `dofile`, `loadfile`, `coroutine` and `print`
are removed. `math`, `string` and `table` remain available. Each mod can use:

| Function | Effect |
|---|---|
| `mod.log(msg)` | print a message tagged with the mod id |
| `mod.set_balance(key, value)` | override a `CONFIG` value (number, bool or string) |
| `mod.get_balance(key)` | read the current `CONFIG` value |
| `mod.set_price(name, price)` | override an item price |
| `mod.register_item(def)` | add or replace an item in the inventory catalogue |
| `mod.on(event, fn)` | subscribe to a game event |

`register_item` accepts the same fields as `ITEMS` in
[scripts.md](scripts.md): `name`, `tab`, `category`, `width`, `height`,
`texture`, `icon`, `icon_width`, `icon_height`, `price`, `frame`, `atlas`,
`animated`, `frames`, `flower`. Texture and frame paths are resolved inside the
mod folder.

```lua
mod.log("Double Money loaded")
mod.set_balance("money_at_cassa", 10)
mod.set_balance("money_at_candy", 2)

mod.register_item({
  name = "neon_box",
  tab = "regular",
  texture = "textures/decor/neon_box.png",
  price = 80,
})

mod.on("object_placed", function(data, dt)
  if data.name == "neon_box" then
    mod.log("neon box placed")
  end
end)
```

### Events

| Event | When it fires |
|---|---|
| `startup` | once, right after the mod is loaded |
| `tick` | every unpaused game frame |
| `npc_update` | every shopper tick; can replace vanilla behaviour |
| `object_placed` | the player places an object |
| `object_removed` | the player removes an object |
| `money_changed` | the cash register total changes |
| `day_start` / `night_start` | day/night phase changes |

Handlers receive `(data, dt)`. Returning `true` (or `{ handled = true }`) marks
the event as handled; for `npc_update` this skips vanilla `npc.lua`, and
mutating `data.state` / `data.timer` updates the shopper. An unknown event name
is ignored and reported as a conflict. A handler error is caught, recorded and
shown in the Mods window; the game keeps running.

## Loading and conflicts

1. Vanilla `config.lua`, `items.lua` and `npc.lua` load into one shared Lua VM.
2. Enabled mods load in the order stored in `data/settings.json`.
3. `load_after` moves dependencies earlier while preserving the rest of the order.
4. Later mods win on duplicate balance keys, prices and item names; the
   conflict is printed and counted in the Mods window.

Because base scripts and mods share a VM, a mod may also define globals such as
`npc_update` directly, but the `mod.*` API is the supported way.

## Enabling mods

The **Mods** window has three entry points: the **Mods** button in the main
menu, **Settings → Mods...** in that same menu, and **Settings → Mods...**
in-game. The
window lists discovered mods with an enable checkbox, title, author, version
and status (`Loaded`, `Disabled`, `Pending restart`, `Error: ...` or
`Needs game X`). When there are more than six mods the list scrolls with the
mouse wheel, by dragging inside the panel or by dragging the scrollbar on the
right. `Escape`, the **Close** button or a click outside the panel returns to
the previous screen.

Toggles are saved both to each `mod.json` (`use`) and to `data/settings.json`
under `mods`:

```json
{
  "vsync": true,
  "zoom_speed": 0.1,
  "music": true,
  "sfx": true,
  "texture_pack": "neon",
  "shader": null,
  "mods": ["double_money", "big_shop"]
}
```

The array order is the load order. Older settings files without `mods` load as
an empty list. Logic changes (balance, items, NPC) are applied on the next
world entry or game restart, as stated in the window. Texture layers apply
immediately, like a texture pack.

A `textures/` folder is added as an overlay layer above the active texture
pack. Later loaded mods have higher priority; missing files fall through to the
pack and then to vanilla. See [texture-packs.md](texture-packs.md).

## Limits

- Lua only; no native plugins.
- No new scenes, screens or render layers.
- No hot reload of logic: toggle and re-enter the world or restart.
- Only the events listed above can be subscribed to.
- Scripts run on the main thread, so an expensive mod can slow the game down.
