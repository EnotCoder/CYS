# Скрипты

[English](../en/scripts.md) · Русский

В `assets/scripts/` лежат два Lua-файла и один dev-инструмент:

- `config.lua` — единственный источник баланса, читается один раз при старте;
- `npc.lua` — логика решений покупателя, вызывается каждый тик;
- `gen_sounds.py` — генератор `.ogg`-эффектов на Python.

Игрового Lua-цикла нет. `config.lua` выполняется один раз в
`BalanceConfig::load`; `npc.lua` вызывается пофункционально из
`ShopperNpc::update`. Если файл отсутствует или сломан, включаются дефолты на
Rust, и игра продолжает работать.

## config.lua

Читается `src/scripts/config.rs::load` в `BalanceConfig`, затем публикуется
глобалом `CONFIG` для остальных скриптов. Правь значения здесь, а не в Rust:
дефолты в Rust существуют только как запасной вариант.

```lua
CONFIG = {
    max_shoppers = 22,
    npc_speed = 3.0,
    start_money = 150,
}
```

| Параметр | Роль |
|---|---|
| `shopper_spawn_interval`, `shopper_spawn_cooldown`, `max_shoppers` | частота, пауза и лимит спавна (`ShopperManager::tick`) |
| `food_regen_tick`, `food_regen_amount` | регенерация еды в коробке (`GameScene::update`) |
| `day_cycle_secs`, `day_secs`, `night_start_secs`, `night_secs`, `fade_secs` | фазы суток и переход (`day_night.rs`) |
| `npc_speed`, `walk_anim_interval`, `npc_fade_speed` | скорость, анимация ходьбы, затухание |
| `spawn_x`, `spawn_y` | точка появления и выхода на тротуаре |
| `cassa_wait_secs`, `candy_wait_secs` | сколько стоять у кассы и у конфет |
| `money_at_cassa`, `money_at_candy`, `candy_chance` | доход и вероятность конфет |
| `max_food_box`, `max_food_rack`, `max_food_candies`, `candies_start_food` | вместимости хранилищ |
| `box_tex_threshold_1`, `box_tex_threshold_2` | пороги текстур коробки по остатку еды |
| `start_money`, `rent_amount`, `rent_interval_secs` | стартовый капитал, аренда, период |
| `object_prices` (таблица) | переопределение цен; остальные берут `Object.price` |
| `font_path` | файл шрифта; применяется при перезапуске |

Согласованность важна: тест
`balance_config_defaults_are_internally_consistent` проверяет порядок фаз
суток, пороги коробки и вместимости конфет. Сломанные значения в `config.lua`
дают понятное падение теста вместо визуального глюка.

## npc.lua

`npc.lua` реализует решения поверх Rust-примитивов: переходы состояний,
таймеры ожидания и выбор маршрута. Номера состояний зеркалят `ShopperState` из
`src/npc/mod.rs`:

| Значение | Состояние |
|---|---|
| `1` | `GoingToRack` |
| `2` | `GoingToCassa` |
| `3` | `AtCassa` |
| `4` | `GoingToCandies` |
| `5` | `AtCandies` |
| `6` | `GoingToExit` |

Жизненный цикл:

1. Спавн на тротуаре, идёт к стеллажу (`start_path`).
2. Берёт еду (`take_food`) и бронирует кассу (`set_busy`).
3. Идёт к кассе, платит (`add_money`), освобождает её (`free_busy`).
4. С шансом `candy_chance` идёт к конфетам, берёт и платит.
5. Идёт к `spawn_x/spawn_y` (`walk_to_exit`) и деспавнится.

Обработка аварий — часть скрипта:

- касса удалена по пути → взять другую или уйти;
- стеллаж удалён или пуст до взятия еды → деспавн;
- магазин закрыт (`npc.exiting`) → покупатель без еды идёт к выходу;
- магазин снова открыт → разворот обратно к стеллажу;
- касса занята → ожидание в очереди;
- витрина конфет удалена или пуста → пропуск и уход.

Та же логика есть на Rust (`update_fallback`). Это не второй источник баланса:
обе версии читают `CONFIG`, а Rust-версия — только fallback, когда скрипт не
загрузился. Меняя состояния, правь обе.

## Lua API

Движок отдаёт одну таблицу `npc` на покупателя. Поля читает и пишет скрипт;
функции — замыкания Rust.

| Вид | Имена |
|---|---|
| поля | `state`, `timer`, `food_taken`, `exiting`, `cassa_x/y`, `candy_x/y`, `rack_x/y`, `done`, `pos_x/y`, `despawn` |
| движение | `walk`, `walk_to_exit`, `start_path`, `set_idle`, `set_walk` |
| объекты | `take_food`, `take_candy`, `cassa_exists`, `rack_exists`, `candy_exists`, `find_any_cassa`, `reroute_to_cassa` |
| экономика | `add_money`, `set_busy`, `free_busy`, `is_busy` |
| отладка | `log` |

`start_path` возвращает `true`, если путь есть; `done` становится true, когда
текущий путь пройден. Скрипт управляет автоматом через `state` и `timer`;
Rust копирует их обратно после каждого вызова.

## gen_sounds.py

Чистый Python-генератор эффектов в стиле 8-бит для `assets/sounds/`.
Синтезирует WAV стандартной библиотекой и конвертирует в OGG через `ffmpeg`.

```sh
python3 assets/scripts/gen_sounds.py
```

Он создаёт `click`, `hover`, `error`, `bell`, `cash`, `pickup`, `candy`,
`stair`, `save` и остальные эффекты. Это dev-инструмент, а не часть сборки:
игра читает только готовые `.ogg` через `core::asset::list_dir`.

## Отладка скриптов

- Синтаксическая ошибка в `npc.lua` отключает скрипт и молча включает
  Rust-fallback; в этом случае `NpcScript::loaded` возвращает `false`.
- `CONFIG` публикуется, только если `config.lua` выполнился; отсутствие файла
  означает дефолты, а не ошибку.
- Внутри скрипта печатать можно через `npc:log("...")`.
