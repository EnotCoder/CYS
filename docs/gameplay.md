# Gameplay

The loop is: place a food source and a register, open the shop, keep it stocked,
pay rent, expand. Everything numeric is in
`assets/scripts/config.lua`; this page explains what each system does.

## Modes

`Tab` cycles `move → interact → build → delete → move`. The mode changes the
cursor texture and the icon in the hotbar.

| Mode | Left click / `F` |
|---|---|
| `move` | nothing on the map; drag moves the camera |
| `interact` | use an object: sell food from a box, refill a rack, enter the basement, refund an arcade machine |
| `build` | place the active hotbar item into the hovered cell |
| `delete` | remove the object, refund half its price |

A successful `build` or `delete` also emits a short burst at the object's
centre. Every particle is a crisp pixel-art square coloured with the item's own
dominant texture colour, so a wooden rack throws brown chips both when placed
and when broken; only the motion preset differs (upward sparkle vs. falling
crumble). The presets live as constants in `ecs/particles.rs`; the particles are
ordinary sprites on `Z_WEATHER` and freeze with the rest of the world while a
modal is open.

`move` is the default on touch devices, because one-finger drag is reserved for
the camera there. On desktop the default is `interact`.

## Objects and placement

`data::ALL_OBJECTS` is the catalogue. Each object has a name, a size in cells
(1×1, 2×2, 1×2…), a texture, an atlas frame and a price. Multi-cell objects are
split into one entity per cell with an offset into the atlas (`rack`, `cassa`,
`candies`, `arcade_machine`, `basement`).

Placement rules (`ecs::placement::can_place_at`):

| Category | Needs | Examples |
|---|---|---|
| floor / regular | floor tile | rack, box, register, table, arcade |
| carpet | floor tile | 5 carpets |
| light | floor tile | lamp, big lamp |
| wall decor | wall tile | welcome sign, FNAF poster, clock |
| outdoor | grass tile | fence, bench, tree |
| flowers | grass tile | flowers |

An object cannot be placed on top of an object of the same category; decor
beats light, light beats carpet for hit-testing, so layering works. Some objects
require a shop purchase first (`data::placement::requires_shop`), others are
free so a new game is playable (`SHOP_EXEMPT`: box, sign, rack, table,
register).

## Economy

| Value | Default | Meaning |
|---|---|---|
| `start_money` | 150 | starting capital |
| `rent_amount` | 15 | deducted every `rent_interval_secs` |
| `rent_interval_secs` | 60 | rent period |
| `money_at_cassa` | 5 | income per food item sold at the register |
| `money_at_candy` | 1 | income per candy |
| `candy_chance` | 0.2 | chance a shopper also visits the candy display |
| `mark_chance` | 0.05 | chance a spawned shopper is Mark instead of a regular character |

Rent is subtracted as `min(money, rent)` — the balance never goes negative.
Bankruptcy triggers when the shop cannot earn (no register, or no food source:
a box, a non-empty rack, or a non-empty candy display) **and** the money left is
less than the cost of buying the missing basics. Then the world pauses, a
`BANKRUPT` screen appears, and `R` (or the Menu button on touch) returns to the
menu.

Removing an object refunds half its price. Buying a shop item deducts the price
once and records it in `ShopOwned`, which is saved per world.

Food flows: a box regenerates `food_regen_amount` every `food_regen_tick`
seconds up to `max_food_box`. Interacting with a box puts food into the active
slot's storage; a rack holds `max_food_rack`, candies hold `max_food_candies`
and start with `candies_start_food`. Shoppers take food from racks and candies
and pay at the register.

## Shoppers

`ShopperManager` spawns shoppers while the shop is open, at most one per
`shopper_spawn_interval` and up to `max_shoppers`, with
`shopper_spawn_cooldown` after a shopper leaves. A shopper spawns on the
sidewalk at `spawn_x/spawn_y`, picks a random non-empty rack, a free register
and optionally the candy display, then:

```
GoingToRack → GoingToCassa → AtCassa → GoingToCandies → AtCandies → GoingToExit
```

Movement is A* on walkable cells, 4-directional, with `npc_speed` cells per
second. Walk frames come from the season-independent character sets and are
swapped every `walk_anim_interval` seconds, unless a texture pack overrides the
fps. With `mark_chance`, a spawn uses Mark's `mark_salesman` textures and the
same state machine; `ShopperManager` allows only one Mark in the shop at a
time, so the next one can enter only after the previous one leaves. The door at
the shop front opens when a `ShopperTag` entity is near and
closes after it leaves; the shopper fades to alpha 0 inside the doorway so the
wall opening does not look like a hole. Empty racks or deleted registers reroute
the shopper; a closed shop sends shoppers without food straight to the exit.

The state machine exists twice: `game/src/npc/mod.rs` (`update_fallback`) and
`assets/scripts/npc.lua`. If the script is missing or fails to load, the Rust
version runs. Both read the same `CONFIG` values.

## Day, night and weather

`DayNightCycle` runs a `day_cycle_secs` loop: day until `day_secs`, a fade to
night at `night_start_secs`, night until `night_secs`, then dawn over
`fade_secs`. `factor()` returns 0..1 dimming for the shader; `time_string()`
shows the in-game clock.

`Season` is a resource and defaults to winter, so a new game starts with snow.
The weather panel changes it, and the whole map image follows
(`map.png`, `spring_map.png`, `autumn_map.png`, `winter_map.png` in
`assets/map/`). Cell categories come from the default image, so gameplay never
changes with the season. `WeatherFx`
adds particles: snow in winter, rain with thunder in autumn, nothing in spring
and summer. Particles are ordinary sprites on `Z_WEATHER`, recycled when they
leave the map. Each season also swaps the background track (`music_spring`,
`music_summer`, `music_autumn`, `music_winter`), and winter/autumn additionally
loop `snow.ogg`/`rain.ogg` as weather ambience.

## Main menu

On the first launch, the logo splash is followed by a short intro cutscene:
darkness, procedural stars, then three quick zooms to the solar system, a
continent and the shop, with `swish.ogg` on each jump. Any click or key skips
it; the skip press is consumed and cannot activate a menu button. Returning
from the game to the menu never replays the splash or the intro.

After the cutscene the main menu shows the game logo and four buttons:
**Play** (world selection), **Settings**, **Mods** and **Quit**. The logo and
buttons fade in with a slight upward slide and a small scale settle, staggered
from top to bottom. **Settings**
opens the same panel as the in-game settings, minus **Back to menu** (there is
nowhere to go back to); `Escape` or a click outside closes it. **Mods** opens
the same scrollable enable/disable panel as **Settings → Mods...** in game;
`Escape` or **Close** returns to the menu. See [mods.md](mods.md).

## Shop, settings, tutorial

- **Shop** (`game/src/ui/shop.rs`) lists every catalogue object except `SHOP_EXEMPT`,
  with a scrollbar and a confirmation dialog. A large portrait of Mark stands
  just left of the panel and bobs gently on a sine wave. Buying adds the name to
  `ShopOwned`; the object then becomes placeable in this world.
- **Settings** (`Esc`, or the gear icon) pauses gameplay: rent, day/night,
  shoppers and weather stop. It holds VSync, zoom speed, music, SFX, a
  **Texture Pack** selector, a **Shader** selector and a **Mods...** button,
  and writes `data/settings.json` immediately. Pack and shader changes are
  applied live; see [texture-packs.md](texture-packs.md) and
  [shaders.md](shaders.md). Mods are enabled in
  [mods.md](mods.md); logic changes apply on the next world entry or restart.
- **Tutorial** opens once per session when the shop is still empty, and can be
  closed with `Esc` or its button. It sways slightly with a sine wave, like the
  shop's Mark portrait.
- **Pause** is implicit: any open modal (shop, weather, settings, mods) freezes
  the world. Only UI input is processed.

## Basement

The basement is a second level (`current_level = 1`) with its own map
(`assets/basement.txt`) and its own object list, cached in `LevelState`. The
stairs object switches levels in both directions. `BasementPlaced` remembers
whether the player has built the entrance, so it survives a reload.

## Input summary

| Action | Desktop | Touch |
|---|---|---|
| place / use / delete | LMB or `F` | tap |
| change mode | `Tab` or mode icon | mode icon |
| inventory | `E` |
| inventory tab | `←` / `→` or a tab icon |
| inventory tab page | arrows beside the tab row | bag icon |
| camera | drag in `move`, arrows | one-finger drag in `move` |
| zoom | wheel, `K`/`L`, pinch | two-finger pinch |
| settings / pause | `Esc` | gear icon, Back button |
| quick save | `Ctrl+S` | autosave on exit |
| menu (after bankruptcy) | `R` | Menu button |
