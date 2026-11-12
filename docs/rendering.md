# Rendering

CYS renders with `wgpu 30` directly: no engine, no sprite-batch crate. The
whole renderer is about a thousand lines in `src/core/`, and its shape is
deliberately boring: one GPU context, two pipelines, three passes.

## GPU context

`WgpuApp` (in `src/core/init.rs`) owns everything long-lived:

- `Instance`, `Surface`, `Adapter`, `Device`, `Queue`;
- `Size` uniform buffer (world) and `UiUniforms` buffer (UI);
- one dynamic storage buffer for per-sprite uniforms;
- light buffer and `DepthBuffer`;
- both render pipelines and their bind groups.

It is created once in `resumed`. On Android `resumed` can fire again after the
app returns from the background; the context is reused and only the surface is
recreated. Rebuilding the whole context there used to break the emulator with
`Failed to find ColorBuffer`.

## Pipelines

| Pipeline | Used for | Blending | Depth compare |
|---|---|---|---|
| `render_pipeline` | map tiles | alpha blend | `LessEqual` |
| `transparent_pipeline` | carpets, light, decor, door, NPC, weather, cursor | `ALPHA_BLENDING` | `Always` |

Both share the same vertex layout: position `[x, y, z]` and `uv`. Depth is
`Depth32Float`, recreated on resize. The transparent pipeline ignores depth on
purpose: gameplay layers are already ordered by `z`, and alpha-cut sprites must
not occlude each other by depth.

## One frame, three passes

1. **Map** — `render_pipeline`, depth cleared. Map tiles are opaque and cover
   the screen.
2. **Transparent** — `transparent_pipeline`, one draw per batched layer in
   z-order. Sprites with `alpha == 0` (for example a shopper inside the doorway)
   are skipped entirely.
3. **UI** — separate uniforms with `night_factor = 0`, so the HUD is never
   darkened by the day/night cycle.

## Layers

Lower `z` draws first. Values live in `src/core/constants.rs`:

```
Z_MAP=0.0 → Z_CARPET=1.0 → Z_LIGHT=1.2 → Z_DECOR=1.5 (Z_DOOR)
→ Z_NPC=1.8 → Z_WEATHER=1.9 → Z_CURSOR=2.0
→ Z_UI=3.0 → Z_UI_TEXT=3.005 → tooltips → dialogs
```

The door lives at `Z_DECOR` (not `Z_MAP`) because its open frame is
transparent; putting it into the opaque pass would render a black hole. It must
stay below `Z_NPC` so the leaf never covers a shopper standing in front of it.

UI sub-layers: `Z_UI_HOTBAR` 3.002, `Z_UI_HOTBAR_CURSOR` 3.003, `Z_UI_TEXT` 3.005, tooltip background 3.01 / text 3.02, dialog
dim 3.03 / panel 3.035 / text 3.04.

## Batching and the sprite cache

Gameplay does not create GPU objects. `get_sprites_by_layer` returns flat
`SpriteRenderData` (position, rotation, texture path, frame, atlas count, scale,
alpha). `render_group` then:

1. builds a cache key with `sprite_cache_key` = layer + path + frame + atlas +
   scale (float hashed by `to_bits`);
2. takes a ready `Sprite` from `EcsAdapter::sprite_cache` or builds it once;
3. writes all `Uniforms` for the batch into one `queue.write_buffer` call,
   padding each 32-byte struct to `min_storage_buffer_offset_alignment`;
4. issues one draw per sprite with a dynamic offset.

The vertex and index buffers are shared (`QUAD_INDICES`), so a sprite costs one
32-byte uniform slot, not four vertices.

`MAX_DYNAMIC_SPRITES` is 65,536, but the real limit is
`max_storage_buffer_binding_size / alignment`. On AMD RADV the binding limit is
64 KiB, so the game may allocate far fewer sprites per frame; `init.rs` clamps
the count with `min` and `render_group` stops when the buffer is full instead of
panicking.

## Textures

`Texture` = `wgpu::Texture` + view + sampler. Two constructors matter:

- `from_path` — decode PNG/JPEG with the `image` crate, upload, fall back to
  `dev_tools/null.png` if the file is missing (a broken asset is visible, not
  fatal);
- `from_rgba` — raw pixels for text and UI.

Atlases are plain grids: `texture_count = [cols, rows]`, `texture_frame =
[col, row]`, and `TEXEL_EPSILON` shrinks UVs by a fraction of a texel to avoid
bleeding between neighbouring frames. Sampling is `Nearest` on purpose: this is
pixel art, and linear filtering turns it into soap.

Virtual paths `tex.png@WxH` stretch one texture over W×H cells; `Sprite::
split_sized_path` parses the suffix after the **last** `@` and ignores invalid
sizes.

## Lighting and day/night

World uniforms carry `map_size` (zoom), `aspect`, camera offset,
`night_factor`, and `light_count`. Lights are `LightData` entries
(`position`, `color + intensity`, `radius`) in a storage buffer. The shader
adds a radial term per light and mixes the result with `night_factor`.

`LightData` must be exactly 64 bytes to match the WGSL `std430` stride. A smaller
Rust struct makes `lights[1..]` read garbage — the classic "second lamp does not
light" bug.

The UI pass uses `UiUniforms` with `night_factor = 0` and `light_count = 0`, so
the interface stays readable at night. The same struct is mirrored into the
global `UI_UNIFORMS`, which UI hit-testing reads to convert NDC to UI units.

## Culling and UI fitting

`sprites(bounds)` receives the visible world rectangle and drops everything
outside it before batching. Both scenes pass it: the game with its scrolling
camera, and the menu with its fixed camera (the menu draws the same shop map —
without culling all ~1500 tiles are submitted every frame and the menu drops
below 60 FPS). `ui_fit_scale` and `ui_visible_half_h` compute a
scale in `[UI_MIN_SCALE, 1.0]` so the whole UI box fits any aspect ratio from
4:3 to 20:9; `ui_top_offset`/`ui_bottom_offset` pin the HUD and hotbar to the
screen edges with `UI_EDGE_MARGIN`.

## Common pitfalls

- **New layer** — add a constant, a `SpriteRenderData` list, a bind in
  `Scene::sprites`, and pass it through `render`; forgetting one leaves sprites
  invisible with no error.
- **Atlas frame** — an out-of-range `[col, row]` samples outside the texture.
  The `every_map_token_resolves_to_an_in_bounds_atlas_frame` test catches map
  tokens.
- **Transparent object below `Z_MAP`** — it will be drawn in the opaque pass
  and lose its alpha.
- **Uniform size** — any struct shared with WGSL must match `std430` padding,
  not just field order.
