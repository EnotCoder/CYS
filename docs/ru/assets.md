# Ассеты

[English](../en/assets.md) · Русский

Всё, что игра читает в рантайме, лежит в `assets/`. Сейчас это 148 PNG и
22 OGG, и все пути начинаются с `assets/`, например
`assets/tex/decor/regular/rack/rack_0.png`.

На Android эти пути упакованы в APK; `core::asset::load_bytes` отрезает
префикс `assets/` перед обращением к `AAssetManager`. Никогда не собирай путь
из `std::env` или абсолютного расположения — только через `core::asset`.

## Дерево

| Путь | Содержимое |
|---|---|
| `assets/tex/map/` | атласы земли: `grass.png` и сезонные варианты, `floor.png`, `wall.png`, `water_shop_wall.png` |
| `assets/tex/decor/regular/` | стеллажи, коробки, касса, стол, аркада, заборы, дверь магазина |
| `assets/tex/decor/carpets/` | атлас ковров |
| `assets/tex/decor/light/` | `lamp.png`, `big_lamp.png` |
| `assets/tex/decor/walldecor/` | плакаты, часы, вывеска welcome |
| `assets/tex/decor/outdoor/` | деревья, скамейки, уличный забор, цветы |
| `assets/tex/characters/` | `player/`, `bob/`, `sasha/` — покой и два кадра ходьбы |
| `assets/tex/ui/` | панели, иконки режимов, курсор, слайдеры, чекбоксы, арт магазина/погоды/настроек |
| `assets/tex/ui/icon_slots/` | иконки инвентаря по категориям (`regular`, `carpets`, `light`, `walldecor`, `outdoor`) |
| `assets/tex/ui/mini_icons/` | мелкие иконки HUD (еда, деньги, часы) |
| `assets/tex/ui/time_of_year/` | кнопки сезонов |
| `assets/tex/dev_tools/` | `null.png` (заглушка текстуры), `black.png` (затемнение) |
| `assets/sounds/` | 22 `.ogg` эффекта и музыка |
| `assets/scripts/` | `config.lua`, `npc.lua`, `gen_sounds.py` |
| `assets/font.otf` | единственный шрифт; путь берётся из `config.lua` |
| `assets/map.txt`, `assets/basement.txt` | карты уровней |
| `assets/icon.png` | исходник иконки приложения |

## Атласы

Текстура — либо одна картинка, либо сетка. Размер сетки хранится в
`texture_count = [cols, rows]`, текущий кадр — в `texture_frame = [col, row]`;
`0,0` — левая верхняя клетка. UV поджимаются на `TEXEL_EPSILON`, чтобы не
зацепить соседний кадр.

Известные размеры атласов:

| Текстура | Сетка |
|---|---|
| `map/grass.png` и сезонные варианты | 4×6 |
| `map/floor.png` | 3×3 |
| `map/wall.png` | 5×5 |
| `decor/regular/shop_door/shop_door.png` | 2×2 |
| `decor/regular/rack/rack.png`, `box/box.png` | кадры анимации объекта |

Кадры анимации — обычные колонки атласа: игра двигает `current_frame` и
читает `frame_paths[i]`, когда текстура собрана из отдельных файлов
(`rack_0.png`, `rack_1.png`, …).

## Пути с размером

Одну текстуру можно растянуть на несколько клеток, не разрезая: добавь `@WxH`
к пути, например `assets/tex/decor/light/big_lamp.png@1x2`.
`Sprite::split_sized_path` берёт суффикс после **последнего** `@`, поэтому в
самом имени файла `@` допустим. Некорректные размеры (`@0x1`, `@2x`, `@-1x2`)
игнорируются, и текстура рисуется в натуральную величину.

## Персонажи

В каталоге персонажа три файла: покой и два кадра ходьбы (`player.png`,
`player_walk_1.png`, `player_walk_2.png`). Покупатели используют тот же
шаблон с `bob` и `sasha`; `ShopperManager` выбирает один из трёх наборов
случайно. Кадры ходьбы меняются каждые `walk_anim_interval` секунд.

## Курсор и иконки режимов

`assets/tex/ui/cursor/` хранит по курсору на режим (`move`, `interact`,
`build`, `delete`) плюс вариант ошибки. `CURSOR_TEX` и `MODE_ICON_TEX` в
`core/constants.rs` индексируются номером режима, поэтому новый режим — это
две текстуры и одна константа.

## Звуки

`AudioEngine` сканирует `assets/sounds/` через `list_dir` и декодирует OGG
через rodio. Имена используются напрямую: `play("click")` ищет `click.ogg`.
Особые случаи:

- `play_music("music")` зацикливает трек; `stop_music` останавливает;
- `play_ambient` для погодных петель (`rain.ogg`), стоп — `stop_ambient`;
- отсутствующие файлы молча пропускаются, поэтому пропавший звук не роняет
  сцену.

Эффекты генерирует `assets/scripts/gen_sounds.py`; музыкальные треки пишутся
отдельно.

## Частицы погоды

Файлов снега и дождя нет. `WeatherFx` создаёт белые пиксели 4×4 для снега и
голубые 2×16 для дождя через `Texture::from_rgba` в рантайме и кэширует спрайт
по синтетическому ключу (`"weather/snow"`). Так папка ассетов остаётся чистой,
а частицы не зависят от разрешения.

## Как добавить ассет

1. Положи файл в нужный каталог категории, имя — `snake_case`.
2. Если это объект, добавь запись в `ALL_OBJECTS` с путём, размером атласа и
   ценой.
3. Если это предмет инвентаря, добавь иконку в
   `assets/tex/ui/icon_slots/icon_slots_object/<категория>/`.
4. Запусти `cargo test`: `every_catalog_object_has_a_resolvable_icon` проверяет,
   что у каждого объекта каталога есть читаемая иконка, а тесты токенов карты —
   атласы.
5. Обнови эту страницу, если появился новый каталог.
