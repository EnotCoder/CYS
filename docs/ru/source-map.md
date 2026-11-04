# Карта исходников (`src/`)

[English](../en/source-map.md) · Русский

По каждому `.rs`-файлу: что реализует и какую роль играет в игре. Структура
папок — по схеме Godot, см. [architecture.md](architecture.md).

Файлы перечислены полными путями: тест `src/tests/docs.rs` следит, чтобы ни
один `.rs` не выпал из этой карты.

---

## `src/main/`

- **`src/main/main.rs`** — десктопная точка входа `[[bin]]`; всё делает
  `cys::run()` из библиотеки.
- **`src/main/lib.rs`** — корень крейта: объявляет 12 модулей через `#[path]`,
  `App` (окно, `WgpuApp`, surface, сцена, ввод, FPS, IME), `render` — полный
  кадр, `ApplicationHandler`, `run()` для десктопа и `android` для Android.
- **`src/main/android.rs`** — `android_main`, скрытие системной панели,
  JNI-мост к `EditText` для ввода имени мира, `poll_text_input` и
  `sync_text_input`.

---

## `src/core/` — ядро wgpu

- **`src/core/mod.rs`** — собирает `asset`, `buffers`, `init`, `pipeline`,
  `render`, `texture`, `constants`, `util` и реэкспортирует основные типы.
- **`src/core/asset.rs`** — единый доступ к файлам: на ПК `std::fs`, на Android
  `AAssetManager` (с отрезанием префикса `assets/`); сохранения и настройки
  пишутся во внутренний каталог приложения; `list_dir` для звуков.
- **`src/core/constants.rs`** — все константы: z-слои, камера и зум, размеры
  окна и UI, пути к текстурам, списки предметов, `MAX_DYNAMIC_SPRITES`,
  `QUAD_INDICES`, служебные числа рендера.
- **`src/core/buffers.rs`** — shader-совместимые `Vertex` и `Uniforms`,
  `DepthBuffer`, `init_buffers` (bind group layouts, dynamic storage buffer
  со смещениями, текстура и сэмплер), depth-stencil состояния.
- **`src/core/init.rs`** — `WgpuApp::new`: instance → surface → adapter →
  device/queue, uniform-буферы `Size` и `UiUniforms`, dynamic storage buffer,
  оба пайплайна, конфигурация surface; глобальные `UI_UNIFORMS`.
- **`src/core/pipeline.rs`** — два пайплайна: обычный (`create_render_pipeline`,
  alpha-blending, depth LessEqual) и прозрачный
  (`create_transparent_pipeline`, Always) плюс раскладка вершин.
- **`src/core/render.rs`** — `render` по слоям: карта (очистка), прозрачный
  проход, UI; `render_group` — кэш спрайтов, батч, `write_buffer`, dynamic
  offsets; передача источников света.
- **`src/core/shaders.wgsl`** — вершинный и фрагментный шейдеры: квады,
  камера, свет, ночное затемнение, UI-пространство.
- **`src/core/texture.rs`** — `Texture` (текстура + view + sampler): из
  PNG/JPEG-байтов, с диска (`from_path`, fallback на `null.png`), из сырых
  RGBA (`from_rgba`, текст и UI).
- **`src/core/util.rs`** — `sprite_cache_key`, `ndc_to_world`, `inventory_index`,
  `slot_icon_path`, `ui_fit_scale`, `ui_top_offset`/`ui_bottom_offset`,
  `ease_out_back`.

---

## `src/data/` — данные, карта, размещение

- **`src/data/mod.rs`** — `Slot`/`Object`, статический каталог `ALL_OBJECTS`,
  `make_slot`, `get_slot_vec`, `object_price`, `shop_item_names`; реэкспорт
  правил размещения; объявляет `door`, `map`, `placement`.
- **`src/data/placement.rs`** — правила `add`/`remove`: категории клеток,
  пересчёт стен (`refresh_walls_around`, `recompute_wall_token`,
  `revert_to_grass`), экономика (цена и возврат половины), компоненты
  `FoodStorage`, `FenceComponent`, `BasementPlaced`, проверки подвала.
- **`src/data/door.rs`** — `tick_door`: дверь открывается, когда покупатель
  рядом (`ShopperTag`), и закрывается после ухода; ресурс `DoorProgress`,
  кадры 2×2-атласа, плавное затухание NPC в проёме.
- **`src/data/map/mod.rs`** — загрузка `map.txt`/`basement.txt` в ECS
  (`load_map_to_ecs`, `load_basement_to_ecs`, `spawn_map_tile`), множества
  стен/пола/травы/проходимых клеток, `shopper_spawn_point`,
  `token_to_texture`, `door_cells`, `door_frame_for_cell`.
- **`src/data/map/pathfinding.rs`** — A*: `Node` (клетка, 4 соседа,
  манхэттенская эвристика), `find_path` через двоичную кучу с восстановлением
  маршрута, `from_world`/`to_world`.

---

## `src/ecs/` — specs и спрайты

- **`src/ecs/mod.rs`** — объявляет `adapter`, `components`, `cursor`,
  `factory`, `group`, `placement`, `sprite`; реэкспортирует адаптер и `Sprite`.
- **`src/ecs/adapter/mod.rs`** — `EcsAdapter`: владеет `World`, кэшем спрайтов,
  текстурами и позиционными множествами карты; регистрация компонентов и
  ресурсов, `add_ui`/`add_ui_sized`, очистка мира, `SpriteRenderData`.
- **`src/ecs/adapter/render.rs`** — `get_sprites_by_layer` (8 z-слоёв с
  отсечением по видимой области), `update_object_textures` (кадры box/rack по
  количеству еды), `update_fence_textures` (забор по соседям),
  `update_sprite_alpha`, `update_transform_position`.
- **`src/ecs/components.rs`** — компоненты и ресурсы: `Transform`,
  `SpriteComponent`, `GroupComponent`, `GroupInfoResource`, `Rotation`,
  `ObjectTag`, `FoodStorage`, `ShopperTag`; `TotalFood`, `Money`, `BusyCassas`,
  `BasementPlaced`, `ShopOwned`, `FenceComponent`, `Season`, `DoorState`,
  `DoorProgress`.
- **`src/ecs/cursor.rs`** — `add_cursor`, `update_cursor_preview`
  (полупрозрачный «призрак» объекта, кадры атласа, маркеры ошибки),
  `clear_cursor_preview`.
- **`src/ecs/factory.rs`** — `create_sprite`: единственный путь создания
  спрайта (`Transform` + `SpriteComponent` из позиции, текстуры, кадра,
  масштаба и альфы).
- **`src/ecs/group.rs`** — многоклеточные объекты: `add_group_object` (на
  каждый тайл — сущность со сдвигом кадра), `delete_group`,
  `find_group_at_position` (декор приоритетнее света, свет — ковра).
- **`src/ecs/placement.rs`** — `can_place_at`: категории требуют подходящие
  клетки (ковры/свет/декор — пол, wall decor — стена, outdoor/цветы — трава),
  запрет объекта поверх той же категории.
- **`src/ecs/sprite.rs`** — `Sprite`: готовый квад (вершины, кадр атласа с
  `TEXEL_EPSILON`, буферы, bind group); `new` для атласа, `from_texture` для
  текста/UI, `split_sized_path` для путей `tex.png@WxH`.

---

## `src/npc/` — покупатель

- **`src/npc/mod.rs`** — `ShopperNpc`: автомат состояний
  (`GoingToRack` → `GoingToCassa` → `AtCassa` → `GoingToCandies` →
  `AtCandies` → `GoingToExit`), спавн с фейдом, API для Lua, `update`
  (скрипт `npc.lua` или `update_fallback` на Rust).
- **`src/npc/interactions.rs`** — `take_food`/`take_candy`, предикаты
  `cassa_exists`, `rack_exists`, `candy_exists`, `find_any_cassa`,
  `reroute_to_cassa`, звуки и текстуры.
- **`src/npc/movement.rs`** — `spawn_path_node`, `set_idle`/`set_walk`,
  `start_path` (A* и запоминание `exit_target`), `walk_toward` (шаг, анимация,
  поворот), `walk_to_exit`.

---

## `src/scenes/` — сцены

- **`src/scenes/mod.rs`** — объявляет `scene_trait`, `scene_manager`,
  `menu_scene`, `game`; реэкспортирует `Scene`, `SceneManager`, `MenuScene`,
  `GameScene`.
- **`src/scenes/scene_trait.rs`** — трейт `Scene` (`on_enter`, `on_exit`,
  `update`, `sprites`, `map_size`, `camera_offset`, `night_factor`, `lights`,
  `ui_size`) и enum `SceneAction`.
- **`src/scenes/scene_manager.rs`** — `SceneManager`: реестр `menu`/`game`,
  единый ECS-мир, `switch_to` (стоп музыки, очистка, `on_enter`),
  `update_fps`, `clear_ecs_world`.
- **`src/scenes/menu_scene.rs`** — `MenuScene`: карта-фон, логотип, кнопки
  Play/Quit, выбор и создание мира, удаление с подтверждением, подсветка,
  запуск по Space/клику, выход по Escape.
- **`src/scenes/game/mod.rs`** — `GameScene`: главный игровой цикл — ввод,
  аренда, банкротство, настройки, реген еды, анимации, день/ночь, покупатели,
  погода, туториал, магазин, hover/tooltip, автосохранение, переключение
  уровней.
- **`src/scenes/game/camera.rs`** — `update_camera`: движение только в режиме
  `move` (drag мышью/пальцем или стрелки), ограничение видимой областью карты.
- **`src/scenes/game/day_night.rs`** — `DayNightCycle`: `tick`, `factor`
  (затемнение 0..1 с рассветом и закатом), `time_string` (часы `ЧЧ:ММ`).
- **`src/scenes/game/hud.rs`** — `GameHud`: «Loading...», подсказка над
  объектом, Food/Money, часы (не чаще раза в секунду), тултип предмета,
  инфо-панель; текст пересоздаётся только при изменении.
- **`src/scenes/game/inventory_input.rs`** — ввод инвентаря: `E`
  (открыть/закрыть), клики по табам и сетке, перенос в активный слот,
  выбор слота хотбара.
- **`src/scenes/game/level.rs`** — уровни: `save_current_level` в
  `LevelState`, `load_level` (магазин ↔ подвал из кэша), `rebuild_ui`,
  `save_to_disk`/`load_from_disk` (`worlds/world_{id}.json`), выход из подвала.
- **`src/scenes/game/shoppers.rs`** — `ShopperManager`: `spawn_shopper`
  (случайные стеллаж/касса/конфеты, три набора текстур), `set_active`,
  `tick` (таймеры спавна, лимит, деспавн, cooldown).
- **`src/scenes/game/tutorial.rs`** — `Tutorial`: панель обучения,
  `open`/`close`, хит-тесты кнопки и панели, лёгкое sin-покачивание.
- **`src/scenes/game/weather_fx.rs`** — `WeatherFx`: частицы дождя, снега и
  листьев, `Effect`, спавн/удаление спрайтов, детерминированный `rand01`.

---

## `src/ui/` — интерфейс

- **`src/ui/mod.rs`** — объявляет `anim`, `components`, `fps`, `inventory`,
  `settings`, `shop`, `system`, `text_input`, `text_renderer`, `weather`;
  реэкспортирует компоненты и систему.
- **`src/ui/components.rs`** — описания элементов: `Panel`, `Button`,
  `Checkbox` (с `sprite_key`), `Slider` (дорожка, ползунок, `dragging`).
- **`src/ui/system.rs`** — создание и удаление UI: `ndc_to_ui`, `is_inside`,
  `is_clicked`, фабрики и деструкторы, `slider_drag`,
  `update_slider_thumb`.
- **`src/ui/inventory.rs`** — `Inventory`: сетка и табы, `items`,
  `switch_tab`, клики по сетке, перенос в хотбар.
- **`src/ui/settings.rs`** — окно настроек: VSync (`vsync_toggled`), слайдер
  скорости зума (`zoom_speed_changed`), музыка/звуки, `open`/`close`,
  `handle_input`, `apply_saved`.
- **`src/ui/shop.rs`** — `Shop`: каталог с прокруткой и скроллбаром,
  `ShopItem`, `ConfirmAction`, диалог подтверждения покупки, `refresh`,
  `row_clicked`, `scroll_input`.
- **`src/ui/weather.rs`** — `Weather`: панель выбора сезона (4 кнопки),
  `selected`, `season_changed`, `open`/`close`, `handle_input`, подсветка.
- **`src/ui/anim.rs`** — `PanelAnim`: открытие/закрытие панелей,
  `AnimPhase`/`AnimEvent`, `start_open`/`start_close`/`cancel`, `tick`,
  захват и сброс исходных трансформов; тайминги 0.28/0.22 с.
- **`src/ui/text_input.rs`** — `TextInput` и глобальный `TEXT_INPUT`: активность,
  фокус, preedit для IME, `push`, `backspace`, `enter`, `take`; используется
  для имени мира.
- **`src/ui/fps.rs`** — `FpsCounter`: пересчёт FPS раз в секунду по числу
  кадров.
- **`src/ui/text_renderer.rs`** — `TextRenderer`: растеризация `ab_glyph`
  в RGBA-текстуру (обводка и заливка), текстовые спрайты, кэш по
  текст+кегль+цвет; `add_text`, `set_text`, `add_text_z`.

---

## `src/input/` — ввод

- **`src/input/mod.rs`** — объявляет `camera`, `cursor`, `interact`, `platform`;
  `do_input` — главная функция кадра: зум, курсор, клики и `F`, исключение
  UI-зон, `Tab` — смена режимов, валидность и превью, возврат слотов.
- **`src/input/camera.rs`** — `handle_zoom`: колесо, `K`/`L`, диапазон
  `ZOOM_MIN..ZOOM_MAX`.
- **`src/input/cursor.rs`** — `handle_mouse_movement` (NDC → мир → клетка,
  клампинг, задержка между тайлами), `update_cursor_validity` (текстура
  ок/ошибка), `update_cursor_preview`; в режиме `move` карта не реагирует.
- **`src/input/interact.rs`** — `try_interact` (box — продажа еды, rack —
  пополнение, basement — смена уровня, аркада — возврат 1), `cycle_mode`,
  `do_interact`, `next_mode`.
- **`src/input/platform.rs`** — `InputSource` (единый интерфейс),
  `DesktopInput` поверх `winit_input_helper`, `TouchInput` (тап, drag, щипок),
  обработка клавиатуры, IME и текстовых событий.

---

## `src/audio/`

- **`src/audio/mod.rs`** — `AudioEngine` на rodio: кэш `sounds/`, синглтон
  через `OnceLock`/`Mutex`; `play`, `play_music`, `play_ambient`,
  `stop_music`, `set_music_enabled`, `set_sfx_enabled`, `init`.

---

## `src/scripts/` — Lua-мост

- **`src/scripts/mod.rs`** — объявляет `config` и `npc`.
- **`src/scripts/config.rs`** — `BalanceConfig`: чтение
  `assets/scripts/config.lua` через mlua с дефолтами при ошибке; тайминги,
  цены, вместимости, экономика; `publish_to_lua`.
- **`src/scripts/npc.rs`** — `NpcScript`: вызов `npc.lua` из Rust, упаковка
  состояния NPC в Lua-таблицу, примитивы (`walk`, `take_food`, `start_path`,
  `busy`, `add_money`, `find_any_cassa`), перенос `state`/`timer` обратно,
  fallback при отсутствии скрипта.

---

## `src/save/`

- **`src/save/mod.rs`** — миры: `WorldMeta`, `WorldManifest`,
  `list_worlds`, `create_world_with_name`, `unique_world_name`, `touch_world`,
  `delete_world`, `WorldSelection`/`SELECTED_WORLD`; настройки
  `GameSettings`, `load_settings`, `save_settings`, глобальный `SETTINGS`.

---

## `src/tests/`

- **`src/tests/mod.rs`** — объявляет наборы тестов и содержит общие проверки:
  режимы, текстовый ввод, чистые функции, константы.
- **`src/tests/helpers.rs`** — заготовки: ECS без GPU, размещение объектов,
  `FakeInput` (реализация `InputSource`), перевод UI-координат в пиксели.
- **`src/tests/data_integrity.rs`** — инварианты констант, каталога предметов,
  баланса, дня/ночи, кэша спрайтов и настроек.
- **`src/tests/placement_rules.rs`** — `can_place_at`, `add`/`remove`:
  категории клеток, конфликты, экономика, подвал, стены.
- **`src/tests/group_tests.rs`** — многоклеточные объекты: раскладка, кадры
  атласа, слои, поиск по позиции, удаление группы.
- **`src/tests/npc_tests.rs`** — покупатель: состояния, движение, еда и
  конфеты, кассы, маршрутизация, фейд-аут, дверь.
- **`src/tests/input_tests.rs`** — ввод через `FakeInput`: зум, хит-тесты UI,
  слайдер, прокрутка магазина, имя мира, затухание у двери, погода.
- **`src/tests/anim_tests.rs`** — `PanelAnim`: открытие, закрытие, отмена,
  текстовые спрайты.
- **`src/tests/docs.rs`** — проверка документации: пары RU/EN, полнота этой
  карты, одинаковые блоки кода, живые ссылки и картинки.
