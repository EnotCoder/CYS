// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  can_place_at — проверка допустимости размещения объекта.
//  Правила: ковры/свет/декор ставятся на пол, настенный декор — на стены,
//  уличные объекты и цветы — на траву; клетка должна быть свободной.
//  Свет — отдельная категория: на клетку с лампой можно ставить и ковры,
//  и мебель; конфликтовать свет может только со светом.
// ========================================================================

use specs::{WorldExt, Join};
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::Transform;
use crate::GroupComponent;

impl EcsAdapter {
    // ====================================================================
    //  can_place_at: Проверяет, можно ли разместить объект.
    //
    //  Правила:
    //   - Обычные предметы (ковры, свет и декор) можно ставить только на пол ('0')
    //   - Настенный декор можно ставить только на блоки стен ('=' и '-')
    //   - Внешний декор и цветы стоят только на предзаданных клетках
    // ====================================================================
    pub fn can_place_at(
        &self,
        x: i32, y: i32,
        width: i32, height: i32,
        is_carpet: bool,
        is_light: bool,
        is_wall_decor: bool,
        is_outdoor: bool,
        is_flower: bool,
    ) -> bool {
        // Категория объекта определяет, какие клетки считаются допустимыми
        // (наборы позиций предварительно вычисляются при загрузке карты).
        if is_flower {
            for i in 0..width {
                for j in 0..height {
                    let cx = x + i;
                    let cy = y + j;
                    if !self.flower_positions.contains(&(cx, cy)) {
                        return false;
                    }
                }
            }
        } else if is_outdoor {
            for i in 0..width {
                for j in 0..height {
                    let cx = x + i;
                    let cy = y + j;
                    if !self.outdoor_positions.contains(&(cx, cy)) {
                        return false;
                    }
                }
            }
        } else if is_wall_decor {
            for i in 0..width {
                for j in 0..height {
                    let cx = x + i;
                    let cy = y + j;
                    if !self.wall_positions.contains(&(cx, cy)) {
                        return false;
                    }
                }
            }
        } else {
            for i in 0..width {
                for j in 0..height {
                    let cx = x + i;
                    let cy = y + j;
                    if !self.floor_positions.contains(&(cx, cy)) {
                        return false;
                    }
                }
            }
        }

        // Вторая проверка: клетка не должна быть занята сущностью другого типа.
        let transforms = self.world.read_storage::<Transform>();
        let group_comps = self.world.read_storage::<GroupComponent>();
        let groups = &self.world.read_resource::<crate::GroupInfoResource>().groups;

        for i in 0..width {
            for j in 0..height {
                let cx = x + i;
                let cy = y + j;

                // Категории независимы: конфликт только «свой на своего» —
                // ковёр не ложится на ковёр, лампа не ставится на лампу,
                // декор не ставится поверх декора; свет и ковёр свободно
                // сочетаются с мебелью друг с другом.
                for (transform, group_comp) in (&transforms, &group_comps).join() {
                    if transform.position[0] as i32 == cx && transform.position[1] as i32 == cy {
                        if let Some(existing) = groups.get(&group_comp.group_id) {
                            let conflicts = if is_carpet {
                                existing.is_carpet
                            } else if is_light {
                                existing.is_light
                            } else {
                                !existing.is_carpet && !existing.is_light
                            };
                            if conflicts {
                                return false;
                            }
                        }
                    }
                }
            }
        }

        true
    }
}
