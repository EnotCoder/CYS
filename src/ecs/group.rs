// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Групповые объекты (multicell): add_group_object, delete_group,
//  find_group_at_position. Здесь создаются многоклеточные сущности —
//  один объект (стол, ковёр, касса) = несколько сущностей с общим group_id.
// ========================================================================

use specs::{WorldExt, Builder};
use std::sync::Arc;
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::{Transform, SpriteComponent};
use crate::{GroupComponent, GroupInfoResource, GroupInfo};

impl EcsAdapter {
    // ====================================================================
    //  add_group_object: Создаёт групповой объект (несколько сущностей).
    //  Ширина/высота определяют, сколько тайлов занимает объект.
    //  Возвращает уникальный group_id, по которому объект можно найти/удалить.
    // ====================================================================
    pub fn add_group_object(
        &mut self,
        x: i32, y: i32,
        width: i32, height: i32,
        texture_path: &str,
        base_frame: [i32; 2],
        tex_count: [i32; 2],
        is_carpet: bool,
        animated: bool,
        frame_paths: &[&str],
    ) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let mut entities = Vec::with_capacity((width * height) as usize);
        // Слой зависит от типа: ковры лежат под декором (см. карту Z-констант).
        let z: f32 = if is_carpet {
            crate::core::constants::Z_CARPET
        } else {
            crate::core::constants::Z_DECOR
        };

        // На каждую клетку объекта создаём отдельную сущность; кадр атласа
        // сдвигается на номер ячейки, чтобы многоклеточная текстура складывалась.
        for i in 0..width {
            for j in 0..height {
                let entity = self
                    .world
                    .create_entity()
                    .with(Transform {
                        position: [(x + i) as f32, (y + j) as f32, z],
                    })
                    .with(SpriteComponent {
                        texture_path: Arc::from(texture_path),
                        texture_frame: [
                            (base_frame[0] + i) % tex_count[0],
                            (base_frame[1] + j) % tex_count[1],
                        ],
                        texture_count: tex_count,
                        scale: 1.0,
                        alpha: 1.0,
                        animated,
                        frame_paths: frame_paths.iter().map(|s| s.to_string()).collect(),
                        current_frame: 0,
                    })
                    .with(GroupComponent { group_id })
                    .build();
                entities.push(entity);
            }
        }

        // Метаданные группы сохраняем в ресурс: размеры и позиция нужны
        // для поиска по координатам и для проверок размещения.
        self.world
            .write_resource::<GroupInfoResource>()
            .groups
            .insert(
                group_id,
                GroupInfo {
                    entities,
                    width,
                    height,
                    pos_x: x,
                    pos_y: y,
                    is_carpet,
                },
            );

        group_id
    }

    // ====================================================================
    //  delete_group: Удаляет все сущности группы и её метаданные
    //  (компоненты вычищаются вручную, т.к. specs не делает это автоматически).
    // ====================================================================
    pub fn delete_group(&mut self, group_id: u32) {
        let group = self
            .world
            .read_resource::<GroupInfoResource>()
            .groups
            .get(&group_id)
            .cloned();

        if let Some(group) = group {
            let entities = self.world.entities();
            let mut transforms = self.world.write_storage::<Transform>();
            let mut sprites = self.world.write_storage::<SpriteComponent>();
            let mut group_comps = self.world.write_storage::<GroupComponent>();

            for entity in group.entities {
                let _ = entities.delete(entity);
                transforms.remove(entity);
                sprites.remove(entity);
                group_comps.remove(entity);
            }

            self.world
                .write_resource::<GroupInfoResource>()
                .groups
                .remove(&group_id);
        }
    }

    // ====================================================================
    //  find_group_at_position: Ищет ID группы по координатам сетки
    //  (декор приоритетнее ковра, если клетки перекрываются).
    // ====================================================================
    pub fn find_group_at_position(&self, x: i32, y: i32) -> Option<u32> {
        let groups = &self.world.read_resource::<GroupInfoResource>().groups;
        let mut carpet_gid: Option<u32> = None;
        for (&gid, group) in groups {
            // Проверяем, попадает ли клетка в прямоугольник группы.
            if x >= group.pos_x
                && x < group.pos_x + group.width
                && y >= group.pos_y
                && y < group.pos_y + group.height
            {
                // Под ковром не может быть декора: декор возвращаем сразу,
                // ковёр запоминаем как запасной вариант.
                if !group.is_carpet {
                    return Some(gid);
                }
                if carpet_gid.is_none() {
                    carpet_gid = Some(gid);
                }
            }
        }
        carpet_gid
    }
}
