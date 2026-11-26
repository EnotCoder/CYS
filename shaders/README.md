# Player shaders

Put each shader in its own folder here, for example `shaders/pixel/`.

```text
shaders/
  pixel/
    shader.json        # optional
    shader.wgsl
```

The folder name is the shader id saved in `data/settings.json`. On Android,
ship shaders under `assets/shaders/` so they are bundled into the APK.

Ready-made `warm/` (Comics, with posterized shading) and `retro/` packs are
included; both are mirrored under `assets/shaders/` for Android and installed
builds. Choose the active shader in Settings → Shader. Both render pipelines
are recreated on the fly; a broken shader falls back to Vanilla and the reason
is shown in Settings. Keep the vanilla `vs_main` / `fs_main` entry points and
bindings — see [docs/shaders.md](../docs/shaders.md) for the full contract.
