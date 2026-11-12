# CYS — documentation

**Create your Shop** is a 2D shop simulator written in Rust on top of `wgpu`.
This folder is the full developer documentation.

| Topic | Page |
|---|---|
| Architecture, frame flow, module layout | [architecture.md](architecture.md) |
| Every source file and what it does | [source-map.md](source-map.md) |
| wgpu pipeline, layers, lighting | [rendering.md](rendering.md) |
| ECS adapter, components, groups | [ecs.md](ecs.md) |
| Economy, shoppers, day/night, weather | [gameplay.md](gameplay.md) |
| Map, saves, `data/settings.json`, worlds | [world-format.md](world-format.md) |
| `config.lua`, `npc.lua`, dev scripts | [scripts.md](scripts.md) |
| Texture and sound paths | [assets.md](assets.md) |
| Building for desktop and Android | [build.md](build.md) |
| Running and writing tests | [testing.md](testing.md) |
| Code style and documentation rules | [contributing.md](contributing.md) |

## Where to start

- **Player / first launch** — root [README](../README.md), section *Build & run*.
- **New to the codebase** — [architecture.md](architecture.md), then
  [source-map.md](source-map.md).
- **Changing rendering** — [rendering.md](rendering.md) and `src/core/`.
- **Balancing the game** — [scripts.md](scripts.md).
- **Changing a save format** — [world-format.md](world-format.md).

## Keeping docs honest

Documentation is part of the build: `cargo test` checks that

- every `src/**/*.rs` file is mentioned in `source-map.md`;
- every relative link and image points at an existing file.

So a renamed file fails CI-level checks instead of rotting silently. Details:
[testing.md](testing.md).
