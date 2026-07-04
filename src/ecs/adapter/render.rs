use specs::{WorldExt, Join};
use std::collections::HashSet;
use crate::ecs::components::{Transform, SpriteComponent, Rotation, ObjectTag, FoodStorage, FenceComponent};
use crate::GroupComponent;
use super::SpriteRenderData;

impl super::EcsAdapter {
    pub fn get_sprites_by_layer(
        &self,
        visible_bounds: Option<(f32, f32, f32, f32)>,
    ) -> (
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
        Vec<SpriteRenderData>,
    ) {
        let transforms = self.world.read_storage::<Transform>();
        let sprites = self.world.read_storage::<SpriteComponent>();
        let rotations = self.world.read_storage::<Rotation>();

        let margin = 2.0;
        let mut map_sprites = Vec::with_capacity(100);
        let mut carpet_sprites = Vec::with_capacity(20);
        let mut decor_sprites = Vec::with_capacity(20);
        let mut npc_sprites = Vec::with_capacity(5);
        let mut cursor_sprites = Vec::with_capacity(1);
        let mut ui_sprites = Vec::with_capacity(10);

        for (transform, sprite, rotation_opt) in (&transforms, &sprites, rotations.maybe()).join() {
            let data = SpriteRenderData {
                position: transform.position,
                rotation: rotation_opt.map(|r| r.rotation).unwrap_or([0.0; 3]),
                texture_path: sprite.texture_path.clone(),
                texture_frame: sprite.texture_frame,
                texture_count: sprite.texture_count,
                scale: sprite.scale,
                alpha: sprite.alpha,
            };

            let z = transform.position[2];
            let should_cull = z == crate::constants::Z_MAP
                || z == crate::constants::Z_CARPET
                || z == crate::constants::Z_DECOR
                || z == crate::constants::Z_NPC;
            if should_cull {
                if let Some((l, r, b, t)) = visible_bounds {
                    let x = transform.position[0];
                    let y = transform.position[1];
                    if x + 1.0 + margin < l || x - margin > r || y + 1.0 + margin < b || y - margin > t {
                        continue;
                    }
                }
            }
            if z == crate::constants::Z_MAP {
                map_sprites.push(data);
            } else if z == crate::constants::Z_CARPET {
                carpet_sprites.push(data);
            } else if z == crate::constants::Z_DECOR {
                decor_sprites.push(data);
            } else if z == crate::constants::Z_NPC {
                npc_sprites.push(data);
            } else if z == crate::constants::Z_CURSOR {
                cursor_sprites.push(data);
            } else {
                ui_sprites.push(data);
            }
        }

        (map_sprites, carpet_sprites, decor_sprites, npc_sprites, cursor_sprites, ui_sprites)
    }

    pub fn update_object_textures(&mut self) {
        let mut updates: Vec<(u32, String)> = Vec::new();
        {
            let tags = self.world.read_storage::<ObjectTag>();
            let foods = self.world.read_storage::<FoodStorage>();
            let groups = self.world.read_storage::<GroupComponent>();
            for (tag, food, group) in (&tags, &foods, &groups).join() {
                let tex = if tag.name == "box" {
                    if food.food_count < 8 {
                        "tex/decor/regular/box/box_0.png"
                    } else if food.food_count < 12 {
                        "tex/decor/regular/box/box_1.png"
                    } else {
                        "tex/decor/regular/box/box_2.png"
                    }
                } else if tag.name == "rack" {
                    if food.food_count == 0 {
                        "tex/decor/regular/rack/rack_0.png"
                    } else {
                        "tex/decor/regular/rack/rack_1.png"
                    }
                } else {
                    continue;
                };
                updates.push((group.group_id, tex.to_string()));
            }
        }
        let group_info = self.world.read_resource::<crate::GroupInfoResource>();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (gid, tex) in &updates {
            if let Some(info) = group_info.groups.get(gid) {
                for &entity in &info.entities {
                    if let Some(sprite) = sprites.get_mut(entity) {
                        sprite.texture_path = tex.clone();
                    }
                }
            }
        }
    }

    pub fn update_fence_textures(&mut self) {
        use std::path::Path;
        let transforms = self.world.read_storage::<Transform>();
        let fences = self.world.read_storage::<FenceComponent>();
        let positions: HashSet<(i32, i32)> = (&fences, &transforms)
            .join()
            .map(|(_, t)| (t.position[0] as i32, t.position[1] as i32))
            .collect();
        let mut sprites = self.world.write_storage::<SpriteComponent>();
        for (fence, transform, sprite) in (&fences, &transforms, &mut sprites).join() {
            let x = transform.position[0] as i32;
            let y = transform.position[1] as i32;
            let right = positions.contains(&(x + 1, y));
            let left = positions.contains(&(x - 1, y));
            let up = positions.contains(&(x, y + 1));
            let down = positions.contains(&(x, y - 1));
            let (dir, fallback) = if fence.name == "street_fence" {
                ("tex/decor/outdoor/street_fence/street_fence", "tex/decor/outdoor/street_fence/street_fence_0_0_0_0.png")
            } else {
                ("tex/decor/regular/fence/fence", "tex/decor/regular/fence/fence_0_0_0_0.png")
            };
            let path = format!("{}_{}_{}_{}_{}.png", dir, up as u8, down as u8, left as u8, right as u8);
            if Path::new(&path).exists() {
                sprite.texture_path = path;
            } else {
                sprite.texture_path = fallback.to_string();
            }
        }
    }
}
