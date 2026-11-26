# Bundled player texture packs

This is the shipped copy of the desktop `texture_pack/` folder and is packed
into the APK as `assets/texture_pack/`, so both platforms use the same pack
ids. No ready-made packs ship yet; the folder is kept so a pack can be added
in one place on both platforms.

```text
assets/texture_pack/
  <id>/
    pack.json          # optional
    textures/          # mirror of assets/tex/
      map/grass.png
```

On desktop the writable `texture_pack/` root takes priority; this folder is
the fallback for installed builds and the only source on Android. The selected
id is stored in `data/settings.json`. See
[docs/texture-packs.md](../../docs/texture-packs.md) for the `pack.json`
format.
