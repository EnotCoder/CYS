# Shader packs

A shader pack replaces the single shared WGSL shader used for the map, decor,
NPCs, weather and UI. It changes appearance only: gameplay, saves and the object
catalogue are untouched. `Vanilla` is the built-in
[`src/core/shaders.wgsl`](../src/core/shaders.wgsl) and is always available as
the first entry in Settings.

## Locations

Both desktop and Android read shaders from `assets/shaders/`. Desktop also
scans `<user data>/assets/shaders/` (the repository's `./assets/shaders/` in
a source checkout), so custom packs can be added without modifying an
installed build. Each immediate subdirectory is a shader
**id** when it contains a `shader.json` or `shader.wgsl` file. Dots at the
start of a name are ignored, ids are deduplicated and sorted, and `Vanilla`
is prepended in Settings.

```text
assets/shaders/
  pixel/
    shader.json
    shader.wgsl
  crt/
    shader.json
    shader.wgsl
```

The repository includes a ready-made `warm/` pack named **Comics**, with bold
colours and posterized shading, and a `retro/` pack. The list is rescanned when
Settings opens, so a folder added while the game is running appears without a
restart. Android packs must be present in `assets/shaders/` at build time.

## Manifest

`shader.json` is optional; without it the id is used as the display name and
`shader.wgsl` is loaded.

```json
{
  "name": "Pixel Art",
  "description": "Crisp pixels without smoothing",
  "author": "someone",
  "version": "1.0.0",
  "file": "shader.wgsl"
}
```

| Field | Meaning |
|---|---|
| `name`, `description`, `author`, `version` | metadata (only `name` is shown in Settings) |
| `file` | WGSL file inside the folder; defaults to `shader.wgsl` |

Paths are confined to the shader folder: `..`, absolute paths and backslashes
are rejected.

## Shader contract

Both pipelines are rebuilt with the same layouts, so a custom shader must keep
the vanilla entry points and bindings:

- entry points: `vs_main`, `fs_main`;
- vertex inputs: `@location(0) position: vec3<f32>`, `@location(1) tex_coord: vec2<f32>`;
- `@group(0) @binding(0)`: storage array of per-sprite `Uniforms`;
- `@group(1) @binding(0)`: sprite texture, `@group(1) @binding(1)`: sampler;
- `@group(2) @binding(0)`: `Size`, `@group(2) @binding(1)`: storage array of `Light`.

See [rendering.md](rendering.md) for the pipeline and layer description. The
simplest safe pack copies `src/core/shaders.wgsl` and edits only the body of
`fs_main`.

## Live switching and fallback

Settings → **Shader** uses `<` / `>` arrows like the texture-pack selector. The
chosen id is stored in `data/settings.json` as `shader`; `null` or a missing
field means Vanilla. The change is applied in the same frame:

1. the WGSL file is read;
2. a new `ShaderModule` and **both** pipelines (`render_pipeline` and
   `transparent_pipeline`) are created;
3. the old pipelines are replaced; the level, camera and ECS are not touched.

Creation runs inside a wgpu validation error scope. If the file is missing, the
manifest is broken, the path is unsafe, or WGSL fails to compile, the game keeps
(or rebuilds) Vanilla pipelines and shows the reason in red in Settings. Bad
shaders cannot crash the game.

## Limits

One shared shader replaces every layer, including UI text; aggressive full-screen
effects will affect the whole frame. This version does not support separate
shaders per layer, post-processing passes, extra render targets or frame
history. Available WGSL features depend on the device, especially on Android.

See [texture-packs.md](texture-packs.md) for art replacement, which works
alongside shader packs.
