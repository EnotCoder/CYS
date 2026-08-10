pub mod placement;

use crate::constants::SLOT_COUNT;

// ========================================================================
//  Slot & Object — предметы инвентаря
// ========================================================================

// Слот активной панели инструментов: ссылается на объект и хранит,
// выбран ли он сейчас (активным может быть только один слот).
pub struct Slot {
    pub obj: Object,
    pub active: bool,
}

// Описание объекта размещения: его размер в клетках и текстура.
// width/height задают занимаемую площадь, texture_frame/texture_count —
// какой кадр атласа текстуры показывать и сколько кадров в атласе.
// frame_paths — отдельные файлы кадров для анимированных объектов.
pub struct Object {
    pub width: i32,
    pub height: i32,
    pub name: &'static str,
    pub path: &'static str,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub animated: bool,
    pub frame_paths: &'static [&'static str],
}

// ========================================================================
//  Все доступные объекты — данные, а не код
// ========================================================================

// Статическая «база данных» всех объектов, которые можно ставить в магазине.
// Это декларативный список данных (размеры, текстуры, категории), а не логика.
const ALL_OBJECTS: &[Object] = &[
    //def object
    Object {
        width: 1, height: 1, name: "box",         path: "tex/decor/regular/box/box_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "sign",         path: "tex/decor/regular/sign.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "rack",         path: "tex/decor/regular/rack/rack_0.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "table",         path: "tex/decor/regular/table.png",
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "cassa",         path: "tex/decor/regular/cassa.png",
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "ice_cream",         path: "tex/decor/regular/ice_cream.png",
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "candies",         path: "tex/decor/regular/candies.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "arcade_machine",
        path: "tex/decor/regular/arcade_machine/a_m_1.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: true,
        frame_paths: &["tex/decor/regular/arcade_machine/a_m_1.png", "tex/decor/regular/arcade_machine/a_m_2.png"],
    },
    //carpets — ковры и панели пола: все используют один атлас carpet.png,
    // но разные кадры (texture_frame) задают конкретный вид.
    Object {
        width: 1, height: 1, name: "blue_carpet", path: "tex/decor/carpets/carpet.png",
        texture_frame: [0, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "red_carpet", path: "tex/decor/carpets/carpet.png",
        texture_frame: [1, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "green_carpet", path: "tex/decor/carpets/carpet.png",
        texture_frame: [0, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "white_carpet", path: "tex/decor/carpets/carpet.png",
        texture_frame: [1, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "black_carpet", path: "tex/decor/carpets/carpet.png",
        texture_frame: [2, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "iron_panel", path: "tex/decor/carpets/carpet.png",
        texture_frame: [0, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "gold_panel", path: "tex/decor/carpets/carpet.png",
        texture_frame: [1, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "diamond_panel", path: "tex/decor/carpets/carpet.png",
        texture_frame: [2, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "welcome", path: "tex/decor/walldecor/welcome/welcome_0.png",
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: true, frame_paths: &["tex/decor/walldecor/welcome/welcome_0.png", "tex/decor/walldecor/welcome/welcome_1.png"],
    },
    Object {
        width: 1, height: 2, name: "fnaf", path: "tex/decor/walldecor/fnaf.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "watch", path: "tex/decor/walldecor/watch.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "fence", path: "tex/decor/regular/fence/fence_0_0_0_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "street_fence", path: "tex/decor/outdoor/street_fence/street_fence_0_0_0_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "tree", path: "tex/decor/outdoor/tree.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "pink_flower", path: "tex/decor/outdoor/pink_flower.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "blue_flower", path: "tex/decor/outdoor/blue_flower.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "yellow_flower", path: "tex/decor/outdoor/yellow_flower.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "red_flower", path: "tex/decor/outdoor/red_flower.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "white_flower", path: "tex/decor/outdoor/white_flower.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 3, height: 2, name: "street_ice_cream", path: "tex/decor/outdoor/street_ice_cream.png",
        texture_frame: [0, 1], texture_count: [3, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "trashcan", path: "tex/decor/outdoor/trashcan.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "bench", path: "tex/decor/outdoor/bench.png",
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    //basement — лестница в подвал (ставится только одна на магазин).
    Object {
        width: 1, height: 2, name: "basement", path: "tex/decor/regular/basement.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
];

// Набор объектов, которые игрок видит в стартовой панели инструментов.
const INITIAL_SLOTS: [&str; SLOT_COUNT] = ["box", "sign", "rack", "table", "cassa"];

// ========================================================================
//  Публичные функции
// ========================================================================

// Создать слот по имени объекта; если имени нет — берём первый объект списка.
pub fn make_slot(name: &str) -> Slot {
    let obj = ALL_OBJECTS.iter()
        .find(|o| o.name == name)
        .unwrap_or(&ALL_OBJECTS[0]);
    Slot {
        obj: Object { ..*obj },
        active: true,
    }
}

// Стартовая панель инструментов: настоящее может быть только первое место.
pub fn get_slot_vec() -> Vec<Slot> {
    INITIAL_SLOTS.iter().enumerate().map(|(i, name)| {
        let obj = ALL_OBJECTS.iter().find(|o| o.name == *name).unwrap();
        Slot {
            obj: Object { ..*obj },
            active: i == 0,
        }
    }).collect()
}

pub use placement::{add, is_carpet_name, is_flower_name, is_outdoor_name, is_wall_decor_name, remove};
