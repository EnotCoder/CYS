# Bundled player texture packs

Texture packs live in `assets/texture_pack/` on both desktop and Android. This
folder is packed into the APK; desktop reads the same folder from the checkout
or an installed build and also scans `<user data>/assets/texture_pack/` for
custom packs. No ready-made packs ship yet; the folder is kept so a pack can
be added in one place on both platforms.

```text
assets/texture_pack/
  <id>/
    pack.json          # optional
    textures/          # mirror of assets/tex/
      map/grass.png
```

The selected id is stored in `data/settings.json`. See
[docs/texture-packs.md](../../docs/texture-packs.md) for the `pack.json`
format.
