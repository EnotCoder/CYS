use specs::{WorldExt, Join};
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::Transform;
use crate::GroupComponent;

impl EcsAdapter {
    // ====================================================================
    //  can_place_at: Проверяет, можно ли разместить объект.
    //
    //  Правила:
    //   - Обычные предметы (ковры и декор) можно ставить только на пол ('0')
    //   - Настенный декор можно ставить только на блоки стен ('=' и '-')
    // ====================================================================
    pub fn can_place_at(
        &self,
        x: i32, y: i32,
        width: i32, height: i32,
        is_carpet: bool,
        is_wall_decor: bool,
        is_outdoor: bool,
        is_flower: bool,
    ) -> bool {
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

        let transforms = self.world.read_storage::<Transform>();
        let group_comps = self.world.read_storage::<GroupComponent>();
        let groups = &self.world.read_resource::<crate::GroupInfoResource>().groups;

        for i in 0..width {
            for j in 0..height {
                let cx = x + i;
                let cy = y + j;

                for (transform, group_comp) in (&transforms, &group_comps).join() {
                    if transform.position[0] as i32 == cx && transform.position[1] as i32 == cy {
                        if let Some(existing) = groups.get(&group_comp.group_id) {
                            if is_carpet {
                                if existing.is_carpet {
                                    return false;
                                }
                            } else if !existing.is_carpet {
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
