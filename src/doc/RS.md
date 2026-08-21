# Rust-исходники (src/)

Документация по каждому `.rs`-файлу: что реализует и его роль в игре.
Структура папок — по схеме Godot (см. `ARCHITECTURE.md`).

---

## src/main/

- **main.rs** — точка входа крейта (`[[bin]]`, корень): объявляет все 12 папок-модулей
  и реэкспорты. `App` реализует winit `ApplicationHandler` (окно, `WgpuApp`, поверхность);
  цикл кадра `render`: `update` сцены → применение `SceneAction` → запись uniform'ов →
  `core::render`. `fn main` запускает event loop.

---

## src/core/ — «ядро»: wgpu-пайплайн и общие утилиты

- **mod.rs** — собирает ядро: объявляет `buffers`, `init`, `pipeline`, `render`,
  `texture`, `constants`, `util` и реэкспортирует `buffers/init/render/texture`.
- **constants.rs** — все константы игры: мировые смещения, Z-слои, зум/камера,
  пути к текстурам (персонажи, курсор, режимы, иконки), размеры окна, списки
  предметов инвентаря, параметры UI/меню, служебные числа рендера
  (MAX_DYNAMIC_SPRITES, QUAD_INDICES).
- **buffers.rs** — shader-совместимые структуры `Vertex` и `Uniforms`, буфер глубины
  `DepthBuffer`, `init_buffers` (bind group layouts: dynamic storage buffer со
  смещениями, текстура+сэмплер) и depth-stencil-состояния для пайплайнов.
- **init.rs** — инициализация wgpu: `WgpuApp::new` (instance → surface → adapter →
  device/queue), uniform-буферы карты (`Size`) и UI (`UiUniforms`), dynamic storage
  buffer под спрайты, оба render-pipeline, конфигурация surface
  (`surface_config` для VSync/resize).
- **pipeline.rs** — два render-pipeline: обычный (`create_render_pipeline`,
  alpha-blending + тест глубины LessEqual) и прозрачный
  (`create_transparent_pipeline`, ALPHA_BLENDING + нот Always), layout вершин.
- **render.rs** — `render`: отрисовка кадра по слоям — map (очистка), transparent
  (ковры+декор+NPC+курсор одним проходом), ui; `render_group`: кэш спрайтов,
  uniform'ы батча `write_buffer`, квады с dynamic offsets.
- **texture.rs** — тип `Texture` (текстура + view + sampler): из байтов PNG/JPEG,
  с диска (`from_path`, fallback на null.png), из сырых RGBA (`from_rgba`, UI/текст).
- **util.rs** — утилиты: `sprite_cache_key` (хеш слоя+текстуры+кадра+масштаба),
  `ndc_to_world` (NDC → мировые координаты), `inventory_index`, `slot_icon_path`.

---

## src/scenes/ — сцены

- **mod.rs** — объявляет `scene_trait`, `scene_manager`, `menu_scene`, `game`;
  реэкспортирует `Scene`, `SceneManager`, `MenuScene`, `GameScene`.
- **scene_trait.rs** — трейт `Scene`: `on_enter`, `update` (возвращает `SceneAction`),
  `sprites` (6 слоёв рендера + visible_bounds), `map_size`, `camera_offset`,
  `night_factor`; enum `SceneAction` (Switch/Quit/VsyncToggle/None).
- **scene_manager.rs** — `SceneManager`: реестр сцен «menu»/«game» и единый ECS-мир;
  `switch_to` (остановка музыки, полная очистка мира, `on_enter`), `update_fps`
  (строка FPS при изменении), `clear_ecs_world`.
- **menu_scene.rs** — `MenuScene`: карта-фон, логотип, кнопки Play/Quit (панели +
  текст), декор из menu_shop.txt; подсветка при наведении, запуск (Space/клик),
  выход (Escape/Quit).
- **game/mod.rs** — `GameScene` — главная игровая сцена: объявляет подмодули
  camera/day_night/hud/inventory_input/level/shoppers. Координация цикла: ввод,
  аренда, банкротство, настройки, реген еды, «поп»-анимации, день/ночь,
  покупатели, анимации, hover/tooltip/статистика; переключение уровней.
- **game/camera.rs** — `update_camera`: позиция камеры (drag средней кнопкой или
  стрелки), ограничение видимой областью карты.
- **game/day_night.rs** — `DayNightCycle`: цикл день/ночь — `tick`, `factor`
  (затемнение 0..1 с плавными переходами рассвета/заката), `time_string`
  (часы «T: ЧЧ:ММ»).
- **game/hud.rs** — `GameHud`: «Loading...», подсказка над объектом, счётчики
  Food/Money, часы (≤1 раз/сек), тултип предмета инвентаря, инфо-панель;
  тексты пересоздаются только при изменении.
- **game/inventory_input.rs** — ввод инвентаря: клавиша E (открыть/закрыть),
  клики по табам категорий, сетке предметов (перенос в активный слот), слотам
  хотбара (выбор активного слота/рамки).
- **game/level.rs** — уровни: `save_current_level` (снимок в `LevelState`),
  `load_level` (магазин ↔ подвал с восстановлением из кэша), `rebuild_ui`,
  сохранение/загрузка save.json (Ctrl+S/Ctrl+L, serde_json), выход из подвала.
- **game/shoppers.rs** — `ShopperManager`: `spawn_shopper` (случайные
  стеллаж/касса/конфеты, 3 набора текстур), `set_active` (открыть/закрыть),
  `tick` (спавн по таймерам/лимиту, обновление и деспавн NPC, cooldown).

---

## src/ecs/ — ECS (specs) и спрайты

- **mod.rs** — собирает модуль: `adapter`, `components`, `cursor`, `factory`,
  `group`, `placement`, `sprite`; реэкспортирует адаптер и тип `Sprite`.
- **adapter/mod.rs** — прослойка между specs и игрой: `EcsAdapter` владеет `World`
  (регистрация компонентов/ресурсов), кэшем спрайтов и позиционными множествами
  карты; методы спрайтов, UI (`add_ui`, `add_ui_sized`), очистка мира; тип
  `SpriteRenderData` — плоские данные для рендера.
- **adapter/render.rs** — методы рендера адаптера: `get_sprites_by_layer`
  (разбиение по 6 Z-слоям с отсечением видимой области),
  `update_object_textures` (кадры box/rack по количеству еды),
  `update_fence_textures` (забор по соседям).
- **components.rs** — компоненты и ресурсы ECS: `Transform`, `SpriteComponent`,
  `GroupComponent`/`GroupInfoResource`/`GroupInfo`, `Rotation`, `ObjectTag`,
  `FoodStorage`; ресурсы `TotalFood`, `Money`, `BusyCassas`, `BasementPlaced`,
  `FenceComponent`.
- **cursor.rs** — методы курсора: `add_cursor` (слой Z_CURSOR),
  `update_cursor_preview` (полупрозрачный «призрак» объекта с кадрами атласа и
  маркерами ошибки), `clear_cursor_preview`.
- **factory.rs** — фабрика `create_sprite`: единственный путь создания спрайта
  (Transform + SpriteComponent из позиции, текстуры, кадра атласа, масштаба, альфы).
- **group.rs** — многоклеточные объекты: `add_group_object` (на каждый тайл —
  сущность с GroupComponent и сдвигом кадра атласа), `delete_group`,
  `find_group_at_position` (декор приоритетнее света, свет — ковра).
- **placement.rs** — `can_place_at`: проверка размещения — категории требуют
  подходящие клетки (ковры/свет/декор — пол, wall decor — стены, outdoor/цветы — трава),
  запрет ставить объект поверх объекта той же категории.
- **sprite.rs** — `Sprite`: готовый к отрисовке квад (вершины, кадр из атласа с
  TEXEL_EPSILON, буферы, bind group текстуры); `new` (атлас) и `from_texture`
  (весь кадр — текст/UI).

---

## src/data/ — данные игры, карта, размещение

- **mod.rs** — объявляет `placement` и `map`; `Slot`/`Object` (размеры, текстура,
  цена, анимация), статическая база `ALL_OBJECTS`, функции `make_slot`,
  `get_slot_vec`, `object_price`; реэкспорт placement (`add`, `remove`, предикаты).
- **placement.rs** — правила размещения/удаления: классификация категорий,
  пересчёт стен (`refresh_walls_around`, `recompute_wall_token`,
  `revert_to_grass`), мини-экономика (цена/возврат половины), установка
  компонентов (FoodStorage, FenceComponent, BasementPlaced), проверки подвала.
- **map/mod.rs** — загрузка map.txt/basement.txt в ECS-землю (`load_map_to_ecs`,
  `load_basement_to_ecs`), множества стен/пола/травы/placeable,
  `load_walkable_cells`, `shopper_spawn_point`, `token_to_texture`.
- **map/pathfinding.rs** — A*: тип `Node` (клетка, 4 соседа, манхэттен-эвристика),
  `find_path` через двоичную кучу с восстановлением маршрута.

---

## src/npc/ — покупатель (ShopperNpc)

- **mod.rs** — `ShopperNpc`: состояние-автомат (GoingToRack/Cassa/Candies/Exit...),
  спавн с фейд-аутом, API для Lua, `update` (Lua`npc.lua` либо fallback
  `update_fallback` на Rust: покупка, кассы, выход, начисление денег).
- **interactions.rs** — взаимодействия с объектами: `take_food`/`take_candy`
  (снятие еды, текстуры, звуки), предикаты `cassa_exists`, `rack_exists`,
  `candy_exists`, `find_any_cassa`, `reroute_to_cassa`.
- **movement.rs** — движение: `spawn_path_node`, `set_idle`/`set_walk` (текстуры),
  `start_path` (A* с запоминанием exit_target), `walk_toward` (шаг + анимация +
  поворот), `walk_to_exit` (к выходу без навигации).

---

## src/input/ — ввод

- **mod.rs** — объявляет `camera`, `interact`, `cursor`; `do_input` — главная
  функция кадра: зум, движение курсора, клики/клавиша F (с исключением UI-зон),
  Tab — смена режима, валидность и превью, возврат слотов/режима/зума.
- **camera.rs** — `handle_zoom`: колесо мыши или K/L, диапазон ZOOM_MIN..MAX.
- **cursor.rs** — `handle_mouse_movement` (NDC→мир→клетка, клампинг, задержка между
  тайлами), `update_cursor_validity` (текстура ок/ошибка),
  `update_cursor_preview` (превью расстановки).
- **interact.rs** — `try_interact` (box — продажа еды, rack — пополнение,
  basement — смена уровня, аркада — возврат 1), `cycle_mode`, `do_interact`.

---

## src/ui/ — интерфейс

- **mod.rs** — объявляет `components`, `fps`, `inventory`, `settings`, `system`,
  `text_renderer`; реэкспортирует `components::*` и `system::*`.
- **components.rs** — описания UI-элементов: `Panel`, `Button`, `Checkbox`
  (с sprite_key), `Slider` (дорожка, ползунок, флаг dragging).
- **system.rs** — создание/уничтожение UI: `ndc_to_ui`, хит-тесты `is_inside`/
  `is_clicked`, фабрики/деструкторы Panel/Button/Checkbox/Slider,
  `slider_drag`, `update_slider_thumb`.
- **inventory.rs** — `Inventory`: открытие/закрытие (сетка + табы), вкладка
  категории (`items`, `switch_tab`), клики по сетке, перенос в слот хотбара.
- **settings.rs** — окно настроек: панель, заголовок, чекбокс VSync (флаг
  `vsync_toggled`), слайдер скорости зума (`zoom_speed_changed`); `open`/`close`,
  `handle_input`.
piccadilly
piccadilly
- **fps.rs** — `FpsCounter`: пересчёт FPS раз в секунду по числу кадров.
- **text_renderer.rs** — `TextRenderer`: растеризация ab_glyph в RGBA-текстуру
  (обводка + заливка), текстовые спрайты, кэш по текст+кегль+цвет; `add_text`,
  `set_text` (инкрементальное обновление), `add_text_fixed`, `add_text_z`.

---

## src/scripts/ — Lua-мост

- **mod.rs** — объявляет `config` (баланс из config.lua) и `npc` (запуск npc.lua).
- **config.rs** — `BalanceConfig`: баланс из scripts/config.lua через mlua с
  дефолтами при ошибке; тайминги, цены, вместимости, экономика;
  `publish_to_lua` публикует CONFIG для других скриптов.
- **npc.rs** — `NpcScript`: вызов scripts/npc.lua из Rust — упаковка состояния NPC
  в Lua-таблицу, примитивы (walk, take_food, start_path, busy, add_money,
  find_any_cassa...), перенос state/timer обратно; fallback при отсутствии скрипта.

---

## src/audio/

- **mod.rs** — аудио-движок на rodio: `AudioEngine` грузит `sounds/` в кэш,
  синглтон (`OnceLock`/`Mutex`); функции `play`, `play_music` (зацикленные треки),
  `stop_music`, `init`.

---

## src/tests/

- **mod.rs** — юнит-тесты крейта, подключается из main.rs через `#[cfg(test)]`;
  сейчас — placeholder-тест.
