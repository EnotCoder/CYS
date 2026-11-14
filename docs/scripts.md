# Scripts

`assets/scripts/` holds three Lua files and one development tool:

- `config.lua` — the single source of balance values, read once at startup;
- `items.lua` — the inventory catalogue: sizes, textures, icons and prices;
- `npc.lua` — shopper decision logic, called every tick;
- `gen_sounds.py` — a Python generator for the `.ogg` sound effects.

There is no Lua game loop. `config.lua` runs once inside
`BalanceConfig::load`; `items.lua` runs once inside `data::catalog`;
`npc.lua` is called function-by-function from `ShopperNpc::update`. If a file is
missing or fails, Rust defaults take over and the game keeps working.

## config.lua

Loaded by `src/scripts/config.rs::load` into `BalanceConfig`, then published as
the global `CONFIG` for other scripts. Edit values here, not in Rust; the
defaults in Rust exist only as a fallback.

```lua
CONFIG = {
    max_shoppers = 22,
    npc_speed = 3.0,
    start_money = 150,
}
```

| Parameter | Role |
|---|---|
| `shopper_spawn_interval`, `shopper_spawn_cooldown`, `max_shoppers` | shopper spawn rate, cooldown and limit (`ShopperManager::tick`) |
| `food_regen_tick`, `food_regen_amount` | box food regeneration (`GameScene::update`) |
| `day_cycle_secs`, `day_secs`, `night_start_secs`, `night_secs`, `fade_secs` | day/night phases and fade (`day_night.rs`) |
| `npc_speed`, `walk_anim_interval`, `npc_fade_speed` | shopper speed, walk animation, fade |
| `spawn_x`, `spawn_y` | sidewalk spawn and exit point |
| `cassa_wait_secs`, `candy_wait_secs` | time spent paying at the register / candy display |
| `money_at_cassa`, `money_at_candy`, `candy_chance` | income and candy probability |
| `max_food_box`, `max_food_rack`, `max_food_candies`, `candies_start_food` | storage capacities |
| `box_tex_threshold_1`, `box_tex_threshold_2` | box texture thresholds by remaining food |
| `start_money`, `rent_amount`, `rent_interval_secs` | starting capital, rent, rent period |
| `object_prices` (table) | per-object price overrides; others use `Object.price` |
| `font_path` | font file; applied at restart |

Consistency matters: the `balance_config_defaults_are_internally_consistent`
test checks the order of day/night phases, box thresholds and candy
capacities. Broken values in `config.lua` produce a readable test failure
instead of a visual glitch.

## items.lua

Loaded by `src/scripts/items.rs::load` into `data::ItemCatalog`, which builds
`ALL_OBJECTS`, the inventory tab lists and per-item icon metadata. The Rust
`BUILTIN_OBJECTS` list in `src/data/mod.rs` is only the fallback when this file
is missing, broken or empty.

```lua
ITEMS = {
    {
        name = "box",
        tab = "regular",
        width = 1, height = 1,
        texture = "assets/tex/decor/regular/box/box_0.png",
        icon = "assets/tex/ui/icon_slots/icon_slots_object/regular/box.png",
        icon_width = 1.0, icon_height = 1.0,
        price = 30,
        frame = {0, 0}, atlas = {1, 1},
        animated = false, frames = {},
    },
}
```

| Field | Role |
|---|---|
| `name` | internal id; used by saves, `config.lua` price overrides and shop unlocks |
| `tab` | inventory tab: `regular`, `carpets`, `walldecor`, `outdoor`, `light`, `new_year` |
| `category` | placement category: `regular`, `carpet`, `walldecor`, `outdoor`, `light`; defaults to the tab when omitted |
| `width`, `height` | footprint in map cells |
| `texture` | atlas texture used in the world |
| `icon` | inventory and hotbar icon |
| `icon_width`, `icon_height` | icon size inside one inventory cell (`1.0` = full cell) |
| `price` | default price; `CONFIG.object_prices` can override it |
| `frame`, `atlas` | atlas frame `{x, y}` and grid size `{cols, rows}` |
| `animated`, `frames` | animation flag and frame files (at least two when animated) |
| `flower` | optional; allows placement on grass |

The catalogue order is the shop order, and each `tab` list is drawn as-is in
the 5×5 inventory grid. `tab` only selects the inventory tab; `category`
controls placement and the z-layer, so an item can live in `tab = "new_year"`
and still place like a carpet (`category = "carpet"`). A missing `category` is
derived from `tab` for old catalogues. Tabs are listed in `TAB_NAMES`
(`src/scripts/items.rs`) and mirrored by `TAB_TEX` (`src/core/constants.rs`);
the row is paginated five icons per page, with arrow buttons to the left of the
first icon and to the right of the last. With the inventory open, `←`/`→` cycle
tabs and the page follows the active tab. Adding, removing or renaming an item
here updates both the inventory and the placement helpers in
`src/data/placement.rs`; `SHOP_EXEMPT` and the starting hotbar remain in Rust.

## npc.lua

`npc.lua` implements decisions on top of Rust primitives: state transitions,
wait timers and route choice. The state numbers mirror `ShopperState` in
`src/npc/mod.rs`:

| Value | State |
|---|---|
| `1` | `GoingToRack` |
| `2` | `GoingToCassa` |
| `3` | `AtCassa` |
| `4` | `GoingToCandies` |
| `5` | `AtCandies` |
| `6` | `GoingToExit` |

Lifecycle:

1. Spawn on the sidewalk, walk to a rack (`start_path`).
2. Take food (`take_food`) and reserve a register (`set_busy`).
3. Walk to the register, pay (`add_money`), free it (`free_busy`).
4. With `candy_chance`, walk to the candy display, take and pay.
5. Walk to `spawn_x/spawn_y` (`walk_to_exit`) and despawn.

Failure handling is part of the script:

- register deleted on the way → pick another one or leave;
- rack deleted or empty before food is taken → despawn;
- shop closed (`npc.exiting`) → a shopper without food goes to the exit;
- shop reopened → turn back to the rack;
- register busy → wait in the queue;
- candy display deleted or empty → skip it and leave.

The same logic exists in Rust (`update_fallback`). That is not a second source
of truth for balance: both read `CONFIG`, and the Rust version is only the
fallback when the script cannot load. Keep them in sync when changing states.

## Lua API

The engine exposes one `npc` table per shopper. Fields are read/written by the
script; functions are Rust closures.

| Kind | Names |
|---|---|
| fields | `state`, `timer`, `food_taken`, `exiting`, `cassa_x/y`, `candy_x/y`, `rack_x/y`, `done`, `pos_x/y`, `despawn` |
| movement | `walk`, `walk_to_exit`, `start_path`, `set_idle`, `set_walk` |
| objects | `take_food`, `take_candy`, `cassa_exists`, `rack_exists`, `candy_exists`, `find_any_cassa`, `reroute_to_cassa` |
| economy | `add_money`, `set_busy`, `free_busy`, `is_busy` |
| debug | `log` |

`start_path` returns `true` if a path exists; `done` becomes true when the
current path is finished. Mutating `state` and `timer` is how the script drives
the machine; Rust copies them back after each call.

## gen_sounds.py

A pure-Python generator for the 8-bit style effects in `assets/sounds/`. It
synthesises WAV with the standard library and converts to OGG with `ffmpeg`.

```sh
python3 assets/scripts/gen_sounds.py
```

It produces `click`, `hover`, `error`, `bell`, `cash`, `pickup`, `candy`,
`stair`, `save` and the other effects. It is a development tool, not part of
the game build: the game only reads the resulting `.ogg` files through
`core::asset::list_dir`.

## Debugging scripts

- A syntax error in `npc.lua` disables the script and silently enables the Rust
  fallback; `NpcScript::loaded` returns `false` in that case.
- A broken `items.lua` prints `[items] ...` and falls back to `BUILTIN_OBJECTS`;
  check the tab name spelling and atlas frame bounds.
- `CONFIG` is only published if `config.lua` ran; a missing file means defaults,
  not an error.
- Use `npc:log("...")` inside the script to print from Lua.
