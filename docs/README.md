# CYS — documentation

**Create your Shop** is a 2D shop simulator written in Rust on top of `wgpu`.
This folder is the full developer documentation. The same texts exist in two
languages; filenames are identical on purpose, so every link has a twin.

Ниже — карта документации. Русский текст — основной, английский — перевод.

| Topic | Русский | English |
|---|---|---|
| Architecture, frame flow, module layout | [architecture.md](ru/architecture.md) | [architecture.md](en/architecture.md) |
| Every source file and what it does | [source-map.md](ru/source-map.md) | [source-map.md](en/source-map.md) |
| wgpu pipeline, layers, lighting | [rendering.md](ru/rendering.md) | [rendering.md](en/rendering.md) |
| ECS adapter, components, groups | [ecs.md](ru/ecs.md) | [ecs.md](en/ecs.md) |
| Economy, shoppers, day/night, weather | [gameplay.md](ru/gameplay.md) | [gameplay.md](en/gameplay.md) |
| Map, saves, `settings.json`, worlds | [world-format.md](ru/world-format.md) | [world-format.md](en/world-format.md) |
| `config.lua`, `npc.lua`, dev scripts | [scripts.md](ru/scripts.md) | [scripts.md](en/scripts.md) |
| Texture and sound paths | [assets.md](ru/assets.md) | [assets.md](en/assets.md) |
| Building for desktop and Android | [build.md](ru/build.md) | [build.md](en/build.md) |
| Running and writing tests | [testing.md](ru/testing.md) | [testing.md](en/testing.md) |
| Code style and documentation rules | [contributing.md](ru/contributing.md) | [contributing.md](en/contributing.md) |

## Where to start

- **Player / first launch** — root [README](../README.md), section *Build & run*.
- **New to the codebase** — [architecture.md](en/architecture.md), then
  [source-map.md](en/source-map.md).
- **Changing rendering** — [rendering.md](en/rendering.md) and `src/core/`.
- **Balancing the game** — [scripts.md](en/scripts.md).
- **Changing a save format** — [world-format.md](en/world-format.md).

## Keeping docs honest

Documentation is part of the build: `cargo test` checks that

- every `docs/ru/*.md` file has an `docs/en/*.md` twin;
- every `src/**/*.rs` file is mentioned in both `source-map.md` files;
- all fenced code blocks are byte-identical between the two languages;
- every relative link and image points at an existing file.

So a renamed file or a diverged translation fails CI-level checks instead of
rotting silently. Details: [testing.md](en/testing.md).
