use specs::{Entity, WorldExt};
use crate::EcsAdapter;
use crate::constants::SLOT_COUNT;

// ========================================================================
//  Slot & Object — предметы инвентаря
// ========================================================================

pub struct Slot {
    pub obj: Object,
    pub active: bool,
}

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

const ALL_OBJECTS: &[Object] = &[
    //def object
    Object {
        width: 1, height: 1, name: "box", path: "tex/decor/box/box_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "sign", path: "tex/decor/sign.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "rack", path: "tex/decor/rack/rack_0.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "table", path: "tex/decor/table.png",
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "cassa", path: "tex/decor/cassa.png",
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 1, name: "ice_cream", path: "tex/decor/ice_cream.png",
        texture_frame: [0, 0], texture_count: [2, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "candies", path: "tex/decor/candies.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "arcade_machine",
        path: "tex/decor/arcade_machine/a_m_1.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: true,
        frame_paths: &["tex/decor/arcade_machine/a_m_1.png", "tex/decor/arcade_machine/a_m_2.png"],
    },
    //carpets
    Object {
        width: 1, height: 1, name: "blue_carpet", path: "tex/decor/carpet.png",
        texture_frame: [0, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "red_carpet", path: "tex/decor/carpet.png",
        texture_frame: [1, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "green_carpet", path: "tex/decor/carpet.png",
        texture_frame: [0, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "white_carpet", path: "tex/decor/carpet.png",
        texture_frame: [1, 1], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "black_carpet", path: "tex/decor/carpet.png",
        texture_frame: [2, 0], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "iron_panel", path: "tex/decor/carpet.png",
        texture_frame: [0, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "gold_panel", path: "tex/decor/carpet.png",
        texture_frame: [1, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "diamond_panel", path: "tex/decor/carpet.png",
        texture_frame: [2, 2], texture_count: [4, 4],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 2, height: 2, name: "welcome", path: "tex/decor/welcome/welcome_0.png",
        texture_frame: [0, 1], texture_count: [2, 2],
        animated: true, frame_paths: &["tex/decor/welcome/welcome_0.png", "tex/decor/welcome/welcome_1.png"],
    },
    Object {
        width: 1, height: 1, name: "fence", path: "tex/decor/fence/fence_0_0_0_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 1, name: "street_fence", path: "tex/decor/street_fence/street_fence_0_0.png",
        texture_frame: [0, 0], texture_count: [1, 1],
        animated: false, frame_paths: &[],
    },
    Object {
        width: 1, height: 2, name: "tree", path: "tex/decor/tree.png",
        texture_frame: [0, 1], texture_count: [1, 2],
        animated: false, frame_paths: &[],
    },
];

const INITIAL_SLOTS: [&str; SLOT_COUNT] = ["box", "sign", "rack", "table", "cassa"];

// ========================================================================
//  Публичные функции
// ========================================================================

pub fn make_slot(name: &str) -> Slot {
    let obj = ALL_OBJECTS.iter()
        .find(|o| o.name == name)
        .unwrap_or(&ALL_OBJECTS[0]);
    Slot {
        obj: Object { ..*obj },
        active: true,
    }
}

pub fn get_slot_vec() -> Vec<Slot> {
    INITIAL_SLOTS.iter().enumerate().map(|(i, name)| {
        let obj = ALL_OBJECTS.iter().find(|o| o.name == *name).unwrap();
        Slot {
            obj: Object { ..*obj },
            active: i == 0,
        }
    }).collect()
}

pub fn is_carpet_name(name: &str) -> bool {
    crate::constants::CARPET_NAMES.contains(&name)
}

pub fn is_wall_decor_name(name: &str) -> bool {
    crate::constants::INV_WALLDECOR.contains(&name)
}

pub fn is_outdoor_name(name: &str) -> bool {
    crate::constants::OUTDOOR_NAMES.contains(&name)
}

// ========================================================================
//  add: Попытка поставить объект из активного слота под курсор
// ========================================================================
pub fn add(ecs: &mut EcsAdapter, slots: &mut Vec<Slot>, act_slot: i32, cursor_entity: Entity) {
    let active_slot = &slots[act_slot as usize].obj;
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    let is_carpet = is_carpet_name(active_slot.name);
    let is_wall_decor = is_wall_decor_name(active_slot.name);
    let is_outdoor = is_outdoor_name(active_slot.name);

    if ecs.can_place_at(
        cursor_x as i32, cursor_y as i32,
        active_slot.width, active_slot.height,
        is_carpet, is_wall_decor, is_outdoor,
    ) {
        ecs.clear_cursor_preview();
        let group_id = ecs.add_group_object(
            cursor_x as i32, cursor_y as i32,
            active_slot.width, active_slot.height,
            active_slot.path,
            active_slot.texture_frame,
            active_slot.texture_count,
            is_carpet,
            active_slot.animated,
            active_slot.frame_paths,
        );
        let first = {
            let groups = ecs.world.read_resource::<crate::GroupInfoResource>();
            groups.groups.get(&group_id).and_then(|g| g.entities.first().copied())
        };
        if let Some(entity) = first {
            use specs::WorldExt;
            ecs.world.write_storage::<crate::ObjectTag>().insert(entity, crate::ObjectTag {
                name: active_slot.name.to_string(),
            }).ok();
            if active_slot.name == "box" {
                ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                    food_count: 0,
                    max_food: 20,
                }).ok();
            } else if active_slot.name == "rack" {
                ecs.world.write_storage::<crate::FoodStorage>().insert(entity, crate::FoodStorage {
                    food_count: 0,
                    max_food: 15,
                }).ok();
            } else if active_slot.name == "fence" {
                ecs.world.write_storage::<crate::FenceComponent>().insert(entity, crate::FenceComponent).ok();
            } else if active_slot.name == "street_fence" {
                ecs.world.write_storage::<crate::StreetFenceComponent>().insert(entity, crate::StreetFenceComponent).ok();
            }
        }
    }
}

// ========================================================================
//  remove: Удаляет группу объектов под курсором
// ========================================================================
pub fn remove(ecs: &mut EcsAdapter, cursor_entity: Entity) -> bool {
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);
    if let Some(group_id) = ecs.find_group_at_position(cursor_x as i32, cursor_y as i32) {
        ecs.delete_group(group_id);
        true
    } else {
        false
    }
}
