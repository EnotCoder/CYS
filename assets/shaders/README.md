# Bundled player shaders

This is the shipped copy of the desktop `shaders/` folder. It is packed into
the APK as `assets/shaders/`, so Android sees the same shader ids as desktop.
Keep the folder names and the `shader.json` / `shader.wgsl` layout in sync with
`shaders/` at the repository root.

```text
assets/shaders/
  retro/
    shader.json
    shader.wgsl
  warm/
    shader.json
    shader.wgsl
```

On desktop the writable `shaders/` root takes priority; this folder is the
fallback for installed builds and the only source on Android. The selected id
is stored in `data/settings.json`. See [docs/shaders.md](../../docs/shaders.md)
for the shader contract.
