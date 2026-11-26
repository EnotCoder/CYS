# Bundled mods

This is the shipped copy of the desktop `mods/` folder and is packed into the
APK. Android reads mods only from here; desktop reads the writable `mods/`
root first and falls back to this folder. Keep both roots in sync: same mod
ids, same `mod.json` fields, same `main.lua` and textures.

- `double_money/` — doubles the payout at the cash register and for candies.
- `big_shop/` — raises starting money and registers a `neon_box` item with its
  own texture, plus an `object_placed` event handler.

These manifests ship with `"use": false`. The writable desktop copies keep the
player's own `"use"` flag, so that value is intentionally different between
the two roots. Enabling happens from the **Mods** button in the main menu or
from **Settings → Mods...**. On Android the enabled state is stored in
`data/settings.json`. See [docs/mods.md](../../docs/mods.md) for the Lua API,
sandbox rules and load order.
