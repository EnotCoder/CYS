// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Данные игры: Slot/Object (предметы инвентаря), статическая база
//  ALL_OBJECTS (все размещаемые объекты), функции make_slot, get_slot_vec,
//  object_price; реэкспорт правил размещения (add, remove, предикаты).
// ========================================================================

pub mod placement;
pub mod map;
pub mod door;

use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

use crate::core::constants::{
    FLOWER_NAMES, INV_CARPETS, INV_LIGHT, INV_OUTDOOR, INV_WALLDECOR,
    SLOT_COUNT, TEX_UI_ICON_SLOTS_OBJECT_DIR,
};

// ========================================================================
//  Slot & Object — предметы инвентаря
// ========================================================================

// Слот активной панели инструментов: ссылается на объект и хранит,
// выбран ли он сейчас (активным может быть только один слот).
pub struct Slot {
    pub obj: Object,
}

// Описание объекта размещения: его размер в клетках и текстура.
// width/height задают занимаемую площадь, texture_frame/texture_count —
// какой кадр атласа текстуры показывать и сколько кадров в атласе.
// frame_paths — отдельные файлы кадров для анимированных объектов.
// price — стоимость установки объекта (монеты игрока).
#[derive(Clone)]
pub struct Object {
    pub width: i32,
    pub height: i32,
    pub name: &'static str,
    pub path: &'static str,
    pub price: i32,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub animated: bool,
    pub frame_paths: &'static [&'static str],
}

// ========================================================================
//  Все доступные объекты — данные, а не код
// ========================================================================

// Встроенная «база данных» объектов — fallback, если items.lua отсутствует
// или повреждён. Рабочий каталог см. в ALL_OBJECTS/catalog().
const BUILTIN_OBJECTS: &[Object] = &[
    //def object
    Object {
        width: 1, height: 1, name: "box",         path: "assets/tex/decor/regular/box/box_0.png",
        price: 30,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "sign",         path: "assets/tex/decor/regular/sign.png",
        price: 5,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "rack",         path: "assets/tex/decor/regular/rack/rack_0.png",
        price: 40,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "table",         path: "assets/tex/decor/regular/table.png",
        price: 15,
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "cassa",         path: "assets/tex/decor/regular/cassa.png",
        price: 60,
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "ice_cream",         path: "assets/tex/decor/regular/ice_cream.png",
        price: 20,
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "candies",         path: "assets/tex/decor/regular/candies.png",
        price: 35,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "arcade_machine",
        path: "assets/tex/decor/regular/arcade_machine/a_m_1.png",
        price: 80,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: true,
        frame_paths: &["assets/tex/decor/regular/arcade_machine/a_m_1.png", "assets/tex/decor/regular/arcade_machine/a_m_2.png"],
    },
    //light — лампы (источники света)
    Object {
        width: 1, height: 1, name: "lamp",         path: "assets/tex/decor/light/lamp.png",
        price: 20,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    //light — большая напольная лампа (1×2 клетки). Одна текстура
    //(texture_count [1,1]) растягивается на 2 клетки через ветку в
    //add_group_object (path@WxH).
    Object {
        width: 1, height: 2, name: "big_lamp",     path: "assets/tex/decor/light/big_lamp.png",
        price: 35,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    //carpets — ковры и панели пола: все используют один атлас carpet.png,
    // но разные кадры (texture_frame) задают конкретный вид.
    Object {
        width: 1, height: 1, name: "blue_carpet", path: "assets/tex/decor/carpets/carpet.png",
        price: 5,
        texture_frame: [0, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "red_carpet", path: "assets/tex/decor/carpets/carpet.png",
        price: 5,
        texture_frame: [1, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "green_carpet", path: "assets/tex/decor/carpets/carpet.png",
        price: 5,
        texture_frame: [0, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "white_carpet", path: "assets/tex/decor/carpets/carpet.png",
        price: 5,
        texture_frame: [1, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "black_carpet", path: "assets/tex/decor/carpets/carpet.png",
        price: 5,
        texture_frame: [2, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "iron_panel", path: "assets/tex/decor/carpets/carpet.png",
        price: 8,
        texture_frame: [0, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "gold_panel", path: "assets/tex/decor/carpets/carpet.png",
        price: 15,
        texture_frame: [1, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "diamond_panel", path: "assets/tex/decor/carpets/carpet.png",
        price: 20,
        texture_frame: [2, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "welcome", path: "assets/tex/decor/walldecor/welcome/welcome_0.png",
        price: 25,
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: true, frame_paths: &["assets/tex/decor/walldecor/welcome/welcome_0.png", "assets/tex/decor/walldecor/welcome/welcome_1.png"],
    },
    Object {
        width: 1, height: 2, name: "fnaf", path: "assets/tex/decor/walldecor/fnaf.png",
        price: 30,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "watch", path: "assets/tex/decor/walldecor/watch.png",
        price: 10,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "fence", path: "assets/tex/decor/regular/fence/fence_0_0_0_0.png",
        price: 5,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "street_fence", path: "assets/tex/decor/outdoor/street_fence/street_fence_0_0_0_0.png",
        price: 8,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "tree", path: "assets/tex/decor/outdoor/tree.png",
        price: 20,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "pink_flower", path: "assets/tex/decor/outdoor/pink_flower.png",
        price: 3,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "blue_flower", path: "assets/tex/decor/outdoor/blue_flower.png",
        price: 3,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "yellow_flower", path: "assets/tex/decor/outdoor/yellow_flower.png",
        price: 3,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "red_flower", path: "assets/tex/decor/outdoor/red_flower.png",
        price: 3,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "white_flower", path: "assets/tex/decor/outdoor/white_flower.png",
        price: 3,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 3, height: 2, name: "street_ice_cream", path: "assets/tex/decor/outdoor/street_ice_cream.png",
        price: 25,
        texture_frame: [0, 1], texture_count: [3, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "trashcan", path: "assets/tex/decor/outdoor/trashcan.png",
        price: 5,
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "bench", path: "assets/tex/decor/outdoor/bench.png",
        price: 12,
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    //basement — лестница в подвал (ставится только одна на магазин).
    Object {
        width: 1, height: 2, name: "basement", path: "assets/tex/decor/regular/basement.png",
        price: 100,
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
];

// ========================================================================
//  Каталог предметов: items.lua + встроенный fallback
// ========================================================================

/// Метаданные предмета, которые нужны интерфейсу, но не логике размещения.
pub struct ItemMeta {
    pub icon: &'static str,
    pub icon_size: [f32; 2],
    pub tab: &'static str,
    pub flower: bool,
}

/// Загруженный каталог: объекты, их иконки и состав вкладок инвентаря.
pub struct ItemCatalog {
    pub objects: Vec<Object>,
    meta: HashMap<String, ItemMeta>,
    tabs: [Vec<&'static str>; 6],
}

static CATALOG: OnceLock<ItemCatalog> = OnceLock::new();

/// Каталог предметов, загружаемый один раз из assets/scripts/items.lua.
pub fn catalog() -> &'static ItemCatalog {
    CATALOG.get_or_init(build_catalog)
}

/// Все объекты каталога (Lua-описания или встроенный fallback).
pub static ALL_OBJECTS: LazyLock<&'static [Object]> =
    LazyLock::new(|| catalog().objects.as_slice());

/// Все объекты каталога без разыменования LazyLock.
pub fn all_objects() -> &'static [Object] {
    catalog().objects.as_slice()
}

/// Метаданные предмета по имени (иконка, размер иконки, вкладка).
pub fn item_meta(name: &str) -> Option<&'static ItemMeta> {
    catalog().meta.get(name)
}

/// Имя вкладки предмета: regular, carpets, walldecor, outdoor или light.
pub fn item_tab(name: &str) -> Option<&'static str> {
    item_meta(name).map(|m| m.tab)
}

/// Список предметов вкладки инвентаря по индексу (0..=5).
/// Неизвестный индекс трактуется как вкладка света — как раньше.
pub fn inventory_tab_names(tab: i32) -> &'static [&'static str] {
    let tabs = &catalog().tabs;
    let light = crate::scripts::items::TAB_NAMES
        .iter()
        .position(|name| *name == "light")
        .unwrap_or(0);
    let index = if (0..tabs.len() as i32).contains(&tab) {
        tab as usize
    } else {
        light
    };
    tabs.get(index).map(Vec::as_slice).unwrap_or(&[])
}

fn build_catalog() -> ItemCatalog {
    match crate::scripts::items::load() {
        Some(defs) => build_from_defs(defs),
        None => build_builtin_catalog(),
    }
}

fn build_from_defs(defs: Vec<crate::scripts::items::ItemDef>) -> ItemCatalog {
    let mut objects = Vec::with_capacity(defs.len());
    let mut meta = HashMap::with_capacity(defs.len());
    let mut tabs: [Vec<&'static str>; 6] = std::array::from_fn(|_| Vec::new());
    let mut seen = std::collections::HashSet::new();

    for def in defs {
        let name = leak(def.name);
        if !seen.insert(name) {
            eprintln!("[items] дубликат имени предмета: {name}");
            continue;
        }
        let builtin = BUILTIN_OBJECTS.iter().find(|o| o.name == name);
        let tab_idx = crate::scripts::items::TAB_NAMES
            .iter()
            .position(|t| *t == def.tab)
            .unwrap_or(0);
        let tab = crate::scripts::items::TAB_NAMES[tab_idx];

        let width = if def.width > 0 { def.width } else { builtin.map_or(1, |o| o.width) };
        let height = if def.height > 0 { def.height } else { builtin.map_or(1, |o| o.height) };
        let path = if def.texture.is_empty() {
            builtin.map_or("", |o| o.path)
        } else {
            leak(def.texture)
        };
        let price = if def.price > 0 { def.price } else { builtin.map_or(1, |o| o.price) };
        let texture_count = if def.atlas[0] > 0 && def.atlas[1] > 0 {
            def.atlas
        } else {
            builtin.map_or([1, 1], |o| o.texture_count)
        };
        let frame_paths = if def.frames.is_empty() {
            builtin.map_or(&[] as &'static [&'static str], |o| o.frame_paths)
        } else {
            leak_slice(def.frames)
        };
        let icon = if def.icon.is_empty() {
            leak(default_icon(tab, name))
        } else {
            leak(def.icon)
        };
        let icon_size = [
            clamp_icon(def.icon_size[0]),
            clamp_icon(def.icon_size[1]),
        ];

        tabs[tab_idx].push(name);
        meta.insert(
            name.to_string(),
            ItemMeta { icon, icon_size, tab, flower: def.flower },
        );
        objects.push(Object {
            width,
            height,
            name,
            path,
            price,
            texture_frame: def.frame,
            texture_count,
            animated: def.animated || frame_paths.len() >= 2,
            frame_paths,
        });
    }

    ItemCatalog { objects, meta, tabs }
}

fn build_builtin_catalog() -> ItemCatalog {
    let mut meta = HashMap::with_capacity(BUILTIN_OBJECTS.len());
    let mut tabs: [Vec<&'static str>; 6] = std::array::from_fn(|_| Vec::new());

    for obj in BUILTIN_OBJECTS {
        let tab_idx = builtin_tab(obj.name);
        let tab = crate::scripts::items::TAB_NAMES[tab_idx];
        tabs[tab_idx].push(obj.name);
        meta.insert(
            obj.name.to_string(),
            ItemMeta {
                icon: leak(default_icon(tab, obj.name)),
                icon_size: [1.0, 1.0],
                tab,
                flower: FLOWER_NAMES.contains(&obj.name),
            },
        );
    }

    ItemCatalog { objects: BUILTIN_OBJECTS.to_vec(), meta, tabs }
}

fn builtin_tab(name: &str) -> usize {
    if INV_CARPETS.contains(&name) {
        1
    } else if INV_WALLDECOR.contains(&name) {
        2
    } else if INV_OUTDOOR.contains(&name) {
        3
    } else if INV_LIGHT.contains(&name) {
        4
    } else {
        0
    }
}

fn default_icon(tab: &str, name: &str) -> String {
    format!("{TEX_UI_ICON_SLOTS_OBJECT_DIR}{tab}/{name}.png")
}

fn clamp_icon(value: f32) -> f32 {
    if value > 0.0 { value.clamp(0.05, 2.0) } else { 1.0 }
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn leak_slice(values: Vec<String>) -> &'static [&'static str] {
    if values.is_empty() {
        return &[];
    }
    let leaked: Vec<&'static str> = values.into_iter().map(leak).collect();
    Box::leak(leaked.into_boxed_slice())
}

// Набор объектов, которые игрок видит в стартовой панели инструментов.
const INITIAL_SLOTS: [&str; SLOT_COUNT] = ["box", "sign", "rack", "table", "cassa"];

// ========================================================================
//  Публичные функции
// ========================================================================

// Создать слот по имени объекта; если имени нет — берём первый объект списка.
pub fn make_slot(name: &str) -> Slot {
    let objects = all_objects();
    let obj = objects.iter()
        .find(|o| o.name == name)
        .unwrap_or(&objects[0]);
    Slot {
        obj: Object { ..*obj },
    }
}

// Стартовая панель инструментов.
pub fn get_slot_vec() -> Vec<Slot> {
    let objects = all_objects();
    INITIAL_SLOTS.iter().map(|name| {
        let obj = objects.iter().find(|o| o.name == *name).unwrap();
        Slot {
            obj: Object { ..*obj },
        }
    }).collect()
}

// Цена установки объекта: переопределение из config.lua (cfg.object_prices)
// или дефолт из данных объекта.
pub fn object_price(name: &str, cfg: &crate::scripts::config::BalanceConfig) -> i32 {
    cfg.object_prices.get(name).copied().unwrap_or_else(|| make_slot(name).obj.price)
}

// Имена объектов, доступных в магазине (все кроме SHOP_EXEMPT).
pub fn shop_item_names() -> Vec<&'static str> {
    all_objects().iter()
        .filter(|o| !crate::core::constants::SHOP_EXEMPT.contains(&o.name))
        .map(|o| o.name)
        .collect()
}

pub use placement::{add, attach_point_light, is_carpet_name, is_flower_name, is_grass_token, is_light_name, is_outdoor_name, is_wall_decor_name, remove};