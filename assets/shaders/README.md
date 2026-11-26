# Bundled player shaders

Shaders live in `assets/shaders/` on both desktop and Android. This folder is
packed into the APK; desktop reads the same folder from the checkout or an
installed build and also scans `<user data>/assets/shaders/` for custom packs.

```text
assets/shaders/
  retro/
    shader.json
    shader.wgsl
  warm/
    shader.json
    shader.wgsl
```

The selected id is stored in `data/settings.json`. See
[docs/shaders.md](../../docs/shaders.md) for the shader contract.
