# Bundled mods

Mods live in `assets/mods/` on both desktop and Android. This folder is packed
into the APK; installed desktop builds read the same path from their assets
and keep player overrides under the user data directory. In a source checkout
the overrides go directly into these files.

- `double_money/` — doubles the payout at the cash register and for candies.
- `big_shop/` — raises starting money and registers a `neon_box` item with its
  own texture, plus an `object_placed` event handler.

These manifests ship with `"use": true`, so both mods are enabled by default.
Toggling a checkbox writes `use` to the user-data copy on an installed build
(`<user data>/assets/mods/<id>/mod.json`), or to this file in a source
checkout. Android assets are read-only, so there the choice is kept in
`data/settings.json`. See [docs/mods.md](../../docs/mods.md) for the Lua API,
sandbox rules and load order.
