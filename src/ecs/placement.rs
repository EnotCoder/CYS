use specs::{WorldExt, Join};
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::Transform;
use crate::GroupComponent;
use crate::constants::*;

impl EcsAdapter {
    // ====================================================================
    //  can_place_at: Проверяет, можно ли разместить объект.
    //
    //  Правила:
    //   - Объект не должен выходить за границы поля
    //   - Ковёр нельзя ставить на другой ковёр
    //   - Декор можно ставить только на ковёр
    //   - Декор нельзя ставить на другой декор
    // ====================================================================
    pub fn can_place_at(
        &self,
        x: i32, y: i32,
        width: i32, height: i32,
        is_carpet: bool,
    ) -> bool {
        if x < GRID_MIN_X as i32 || x + width > GRID_MAX_X as i32 + GRID_BOUNDARY_ADJUST
            || y < GRID_MIN_Y as i32 || y + height > GRID_MAX_Y as i32 + GRID_BOUNDARY_ADJUST
        {
            return false;
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
