# Player texture packs

Put each pack in its own folder here, for example `texture_pack/neon/`.

```text
texture_pack/
  neon/
    pack.json          # optional
    textures/          # mirror of assets/tex/
      map/grass.png
```

The folder name is the pack id saved in `data/settings.json`. Mirror each pack
under `assets/texture_pack/` so it is bundled into the APK; desktop reads this
writable root first and falls back to the shipped copy.

Choose the active pack in Settings → Texture Pack. See
[docs/texture-packs.md](../docs/texture-packs.md) for the full `pack.json`
format, override fields and limitations.
