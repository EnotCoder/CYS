# Архитектура (CYS)

[English](../en/architecture.md) · Русский

Файловая система исходников организована по принципу движка Godot:
в `src/` ровно 12 папок верхнего уровня, у каждой есть `mod.rs`, который
собирает подмодули. Всё остальное — ассеты и артефакты сборки.

```
src/
├── main/
├── core/
├── scenes/
├── ecs/
├── data/
├── ui/
├── input/
├── npc/
├── audio/
├── scripts/
├── save/
└── tests/
```

- `main/` — точка входа: `main.rs` (корень крейта), `lib.rs` (`App`, winit-цикл),
  `android.rs` (Android-точка входа и системная клавиатура).
- `core/` — ядро wgpu: `init`, `pipeline`, `render`, `texture`, `buffers`,
  `shaders.wgsl`, `constants`, `util`, `asset` (чтение файлов на ПК и Android).
- `scenes/` — меню и игра: уровень, камера, день/ночь, HUD, покупатели,
  туториал, погодные эффекты.
- `ecs/` — адаптер `specs`: компоненты, фабрика спрайтов, группы, курсор,
  правила размещения, спрайт-квад.
- `data/` — каталог предметов, размещение/снос, карта и A*, дверь.
- `ui/` — панели, кнопки, чекбоксы, слайдеры, инвентарь, настройки, магазин,
  погода, анимации панелей, текстовый ввод, растеризация текста.
- `input/` — общий трейт ввода, камера, курсор, взаимодействие.
- `npc/` — покупатель: состояния, движение, взаимодействие с объектами.
- `audio/` — rodio-движок и кэш звуков.
- `scripts/` — Lua-мост: баланс `config.lua` и поведение `npc.lua`.
- `save/` — миры, манифест, `settings.json`.
- `tests/` — юнит-тесты крейта.

Ассеты лежат в `assets/` (`assets/tex/`, `assets/sounds/`, `assets/scripts/`,
файлы карт) и в `src/` не входят. На Android они упаковываются в APK.

## Крейт

Cargo собирает две цели из одного крейта:

| Цель | Путь | Роль |
|---|---|---|
| `lib` (`rlib` + `cdylib`) | `src/main/lib.rs` | вся логика, `run()`, `android_main` |
| `bin` | `src/main/main.rs` | десктопная точка входа, зовёт `cys::run()` |

`lib.rs` объявляет 12 модулей через `#[path = "../<модуль>/mod.rs"]`: папки
сохраняют Godot-структуру, а Cargo видит плоское дерево модулей. `main.rs`
намеренно крошечный — только делегирование в библиотеку.

## Один кадр

`App` реализует `ApplicationHandler` из winit:

```
new_events          → input.step()
resumed             → window + WgpuApp + surface
window_event        → input.process_window_event()
  RedrawRequested   → App::render()
about_to_wait       → input.end_step(); request_redraw()
suspended           → surface = None
```

`App::render` — конвейер кадра:

1. `scene.update(...)` — ввод, геймплей, анимации. Возвращает `SceneAction`.
2. Применение действия: `Switch`, `Quit`, `VsyncToggle`, `None`.
3. Обновление состояния IME (текстовый ввод на ПК и Android).
4. Тик FPS-счётчика и обновление надписи FPS.
5. Запись world-uniform'ов (`Size`) и UI-uniform'ов (`UiUniforms`).
6. `scene.sprites(...)` и `scene.lights(...)` — сбор данных по слоям, уже
   отсечённых по видимой области.
7. `core::render` — проход карты, прозрачный проход
   (ковры/свет/декор/NPC/погода/курсор) и проход UI.

## Сцены

`SceneManager` владеет одним ECS-миром и реестром сцен (`menu`, `game`).
`switch_to` останавливает музыку, очищает мир и вызывает `on_enter`.
Трейт `Scene`:

| Метод | Смысл |
|---|---|
| `on_enter` | создать сущности, UI и состояние сцены |
| `update` | один тик; возвращает `SceneAction` |
| `sprites` | 8 списков спрайтов (map, carpet, light, decor, NPC, weather, cursor, UI) |
| `map_size`, `camera_offset`, `night_factor`, `lights`, `ui_size` | uniform'ы рендера |

`SceneAction` — единственный канал от сцены к `App`:
`Switch(&'static str)`, `Quit`, `VsyncToggle(bool)`, `None`.

## ECS и поток данных

`EcsAdapter` оборачивает `specs::World` и один раз регистрирует все компоненты
и ресурсы. Игровой код не работает со `specs` напрямую в рендере: он зовёт
`create_sprite`, `add_group_object`, `add_ui`, а затем читает плоские
`SpriteRenderData` из `get_sprites_by_layer`.

`data::ALL_OBJECTS` — единый каталог предметов (имя, размер, текстура, цена).
`data::placement` решает, можно ли ставить объект в клетку; `data::add/remove`
устанавливают и снимают компоненты.

## Абстракция ввода

`input::platform::InputSource` скрывает различия ПК и телефона:

- `DesktopInput` переадресует в `winit_input_helper`;
- `TouchInput` превращает тап, перетаскивание одним пальцем и щипок двумя
  в те же события мыши и колеса.

`App` хранит `Box<dyn InputSource>` и передаёт его сценам, поэтому игровой код
не знает о платформе. События клавиатуры и IME накапливаются за кадр;
`step()` и `end_step()` задают границу кадра.

## Сохранения

`save/` — список миров в стиле Minecraft:

- `worlds/manifest.json` — следующий id и метаданные (`WorldMeta`);
- `worlds/world_{id}.json` — полный снимок уровня;
- `settings.json` — VSync, скорость зума, музыка и звуки.

`save::SELECTED_WORLD` — статика, потому что `SceneAction` не умеет носить
параметры: меню записывает выбор перед `Switch("game")`. На Android те же
функции пишут во внутренний каталог приложения.

## Слои рендера

Меньший `z` рисуется раньше. Значения — в `core::constants`:

```
Z_MAP=0.0 → Z_CARPET=1.0 → Z_LIGHT=1.2 → Z_DECOR=1.5 (Z_DOOR)
→ Z_NPC=1.8 → Z_WEATHER=1.9 → Z_CURSOR=2.0
→ Z_UI=3.0 → Z_UI_TEXT=3.005 → tooltips → dialogs
```

## С чего читать

1. `src/main/lib.rs` и `src/main/main.rs` — запуск.
2. `src/scenes/scene_trait.rs` и `scene_manager.rs` — контракт кадра.
3. `src/scenes/game/mod.rs` — игровой цикл.
4. `src/core/render.rs` и `src/core/init.rs` — как появляются пиксели.
5. `src/data/mod.rs` — что игрок может поставить.
