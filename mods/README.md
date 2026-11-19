# Bundled mods

This folder holds example player mods for the desktop build. Each subdirectory
is a mod id and needs a [mod.json](../docs/mods.md) manifest plus a Lua entry
point (`main.lua` by default). The same mods are bundled under `assets/mods/`
for Android.

- `double_money/` — doubles the payout at the cash register and for candies.
- `big_shop/` — raises starting money and registers a `neon_box` item with its
  own texture, plus an `object_placed` event handler.

Both ship with `"use": false` and stay disabled until enabled from the **Mods**
button in the main menu (between Play and Quit) or **Settings → Mods...** in
game. The panel scrolls when there are many mods; toggling a checkbox rewrites
`use` in these manifests on desktop, while Android keeps the choice in
`settings.json`. See [docs/mods.md](../docs/mods.md) for the Lua API, sandbox
rules, load order and settings format.
