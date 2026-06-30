use specs::Entity;
use crate::ecs::adapter::EcsAdapter;
use crate::constants::Z_CURSOR;

impl EcsAdapter {
    pub fn add_cursor(&mut self, x: f32, y: f32, texture_path: &str) -> Entity {
        crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_CURSOR,
            texture_path, [0, 0], [1, 1], 1.0, 1.0,
        )
    }

    pub fn update_cursor_preview(
        &mut self,
        cursor_x: f32, cursor_y: f32,
        width: i32, height: i32,
        valid: bool,
        tex_path: &str,
        base_frame: [i32; 2],
        tex_count: [i32; 2],
    ) {
        self.clear_cursor_preview();
        let cur_tex = if valid { crate::constants::CURSOR_TEX[1] } else { crate::constants::CURSOR_ERR_TEX };
        let ghost_alpha = if valid { 0.5 } else { 0.3 };

        for i in 0..width {
            for j in 0..height {
                if i != 0 || j != 0 {
                    let entity = crate::ecs::factory::create_sprite(
                        &mut self.world,
                        cursor_x + i as f32, cursor_y + j as f32, Z_CURSOR,
                        cur_tex, [0, 0], [1, 1], 1.0, 1.0,
                    );
                    self.cursor_preview.push(entity);
                }
                let entity = crate::ecs::factory::create_sprite(
                    &mut self.world,
                    cursor_x + i as f32, cursor_y + j as f32, Z_CURSOR,
                    tex_path,
                    [
                        (base_frame[0] + i) % tex_count[0],
                        (base_frame[1] + j) % tex_count[1],
                    ],
                    tex_count,
                    1.0, ghost_alpha,
                );
                self.cursor_preview.push(entity);
            }
        }
    }

    pub fn clear_cursor_preview(&mut self) {
        self.delete_entities(&self.cursor_preview);
        self.cursor_preview.clear();
    }
}
