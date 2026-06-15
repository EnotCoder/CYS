use specs::Entity;
use crate::EcsAdapter;

// ========================================================================
//  Slot & Object — предметы инвентаря
// ========================================================================

/// Один слот инвентаря
pub struct Slot {
    pub obj: Object,
    pub active: bool,
}

/// Описание игрового объекта (текстура, размер, положение в атласе)
pub struct Object {
    pub width: i32,
    pub height: i32,
    pub name: String,
    pub path: String,
    /// Кадр текстуры в атласе [column, row]
    pub texture_frame: [i32; 2],
    /// Размер атласа [columns, rows]
    pub texture_count: [i32; 2],
}

// ========================================================================
//  add: Попытка поставить объект из активного слота под курсор
// ========================================================================
pub fn add(ecs: &mut EcsAdapter, slots: &mut Vec<Slot>, act_slot: i32, cursor_entity: Entity) {
    let active_slot = &slots[act_slot as usize].obj;
    let (cursor_x, cursor_y) = ecs.get_transform_position(cursor_entity);

    // Определяем, является ли объект ковром (по имени)
    let is_carpet = matches!(active_slot.name.as_str(), "carpet" | "red_carpet");

    // Проверяем возможность размещения и создаём группу
    if ecs.can_place_at(
        cursor_x as i32,
        cursor_y as i32,
        active_slot.width,
        active_slot.height,
        is_carpet,
    ) {
        ecs.clear_cursor_preview();
        ecs.add_group_object(
            cursor_x as i32,
            cursor_y as i32,
            active_slot.width,
            active_slot.height,
            &active_slot.path,
            active_slot.texture_frame,
            active_slot.texture_count,
            is_carpet,
        );
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

// ========================================================================
//  get_slot_vec: Возвращает начальный набор предметов (6 слотов)
// ========================================================================
pub fn get_slot_vec() -> Vec<Slot> {
    vec![
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("box"),
                path: String::from("tex/decor/box.png"),
                texture_frame: [0, 0], texture_count: [1, 1],
            },
            active: true,
        },
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("carpet"),
                path: String::from("tex/decor/carpet.png"),
                texture_frame: [0, 0], texture_count: [2, 2],
            },
            active: false,
        },
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("sign"),
                path: String::from("tex/decor/sign.png"),
                texture_frame: [0, 0], texture_count: [1, 1],
            },
            active: false,
        },
        Slot {
            obj: Object {
                width: 1, height: 2, name: String::from("rack"),
                path: String::from("tex/decor/rack.png"),
                // Rack — 1x2: кадр (col=0, row=1) в атласе 1x2
                texture_frame: [0, 1], texture_count: [1, 2],
            },
            active: false,
        },
        Slot {
            obj: Object {
                width: 2, height: 1, name: String::from("table"),
                path: String::from("tex/decor/table.png"),
                // Table — 2x1: кадр (col=0, row=0) в атласе 2x1
                texture_frame: [0, 0], texture_count: [2, 1],
            },
            active: false,
        },
        Slot {
            obj: Object {
                width: 1, height: 1, name: String::from("red_carpet"),
                path: String::from("tex/decor/carpet.png"),
                // Красный ковёр — второй кадр в атласе carpet.png (col=1, row=0)
                texture_frame: [1, 0], texture_count: [2, 2],
            },
            active: false,
        },
    ]
}
