use specs::{WorldExt, Builder};
use crate::ecs::adapter::EcsAdapter;
use crate::ecs::components::{Transform, SpriteComponent};
use crate::constants::Z_CURSOR;

impl EcsAdapter {
    // ====================================================================
    //  add_cursor: Создаёт курсор (z=Z_CURSOR, над decor)
    // ====================================================================
    pub fn add_cursor(&mut self, x: f32, y: f32, texture_path: &str) -> specs::Entity {
        self.world
            .create_entity()
            .with(Transform { position: [x, y, Z_CURSOR] })
            .with(SpriteComponent {
                texture_path: texture_path.to_string(),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: 1.0,
                alpha: 1.0,
            })
            .build()
    }

    // ====================================================================
    //  update_cursor_preview: Показывает размер объекта под курсором
    // ====================================================================
    pub fn update_cursor_preview(
        &mut self,
        cursor_x: f32, cursor_y: f32,
        width: i32, height: i32,
        valid: bool,
    ) {
        self.clear_cursor_preview();

        let tex = if valid { crate::constants::CURSOR_TEX[1] } else { crate::constants::CURSOR_ERR_TEX };

        for i in 0..width {
            for j in 0..height {
                if i == 0 && j == 0 {
                    continue;
                }
                let entity = self.world
                    .create_entity()
                    .with(Transform {
                        position: [cursor_x + i as f32, cursor_y + j as f32, Z_CURSOR],
                    })
                    .with(SpriteComponent {
                        texture_path: tex.to_string(),
                        texture_frame: [0, 0],
                        texture_count: [1, 1],
                        scale: 1.0,
                        alpha: 1.0,
                    })
                    .build();
                self.cursor_preview.push(entity);
            }
        }
    }

    // ====================================================================
    //  clear_cursor_preview: Удаляет превью-спрайты курсора
    // ====================================================================
    pub fn clear_cursor_preview(&mut self) {
        self.delete_entities(&self.cursor_preview);
        self.cursor_preview.clear();
    }
}
