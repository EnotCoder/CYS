use specs::{WorldExt, Builder};
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::{Transform, SpriteComponent};
use crate::{GroupComponent, GroupInfoResource, GroupInfo};

impl EcsAdapter {
    // ====================================================================
    //  add_group_object: Создаёт групповой объект (несколько сущностей).
    //  Ширина/высота определяют, сколько тайлов занимает объект.
    // ====================================================================
    pub fn add_group_object(
        &mut self,
        x: i32, y: i32,
        width: i32, height: i32,
        texture_path: &str,
        base_frame: [i32; 2],
        tex_count: [i32; 2],
        is_carpet: bool,
    ) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let mut entities = Vec::with_capacity((width * height) as usize);
        let z: f32 = if is_carpet {
            crate::constants::Z_CARPET
        } else {
            crate::constants::Z_DECOR
        };

        for i in 0..width {
            for j in 0..height {
                let entity = self
                    .world
                    .create_entity()
                    .with(Transform {
                        position: [(x + i) as f32, (y + j) as f32, z],
                    })
                    .with(SpriteComponent {
                        texture_path: texture_path.to_string(),
                        texture_frame: [
                            (base_frame[0] + i) % tex_count[0],
                            (base_frame[1] + j) % tex_count[1],
                        ],
                        texture_count: tex_count,
                        scale: 1.0,
                    })
                    .with(GroupComponent { group_id })
                    .build();
                entities.push(entity);
            }
        }

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
    // ====================================================================
    pub fn find_group_at_position(&self, x: i32, y: i32) -> Option<u32> {
        for (&gid, group) in &self
            .world
            .read_resource::<GroupInfoResource>()
            .groups
        {
            if x >= group.pos_x
                && x < group.pos_x + group.width
                && y >= group.pos_y
                && y < group.pos_y + group.height
            {
                return Some(gid);
            }
        }
        None
    }
}
