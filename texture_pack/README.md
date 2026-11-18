# Player texture packs

Put each pack in its own folder here, for example `texture_pack/neon/`.

```text
texture_pack/
  neon/
    pack.json          # optional
    textures/          # mirror of assets/tex/
      map/grass.png
```

The folder name is the pack id saved in `data/settings.json`. On Android, ship
packs under `assets/texture_pack/` instead so they are bundled into the APK.

Choose the active pack in Settings → Texture Pack. See
[docs/texture-packs.md](../docs/texture-packs.md) for the full `pack.json`
format, override fields and limitations.
