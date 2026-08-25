// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Курсор: add_cursor (спрайт курсора на слое Z_CURSOR),
//  update_cursor_preview (полупрозрачный «призрак» размещаемого объекта
//  с кадрами атласа и маркерами ошибки), clear_cursor_preview.
// ========================================================================

use specs::Entity;
use crate::ecs::adapter::EcsAdapter;
use crate::core::constants::Z_CURSOR;

impl EcsAdapter {
    // Создаёт спрайт курсора мыши на слое Z_CURSOR (поверх игровых слоёв).
    pub fn add_cursor(&mut self, x: f32, y: f32, texture_path: &str) -> Entity {
        crate::ecs::factory::create_sprite(
            &mut self.world, x, y, Z_CURSOR,
            texture_path, [0, 0], [1, 1], 1.0, 1.0,
        )
    }

    // Перерисовывает "призрак" размещаемого объекта над сеткой.
    // Каждая клетка объекта получает копию текстуры с нужным кадром атласа,
    // а отдельный маркер обозначает заполненность/ошибку (зависит от `valid`).
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
        // Цвет и прозрачность призрака зависят от допустимости позиции.
        let cur_tex = if valid { crate::core::constants::CURSOR_TEX[1] } else { crate::core::constants::CURSOR_ERR_TEX };
        let ghost_alpha = if valid { 0.5 } else { 0.3 };

        // Если объект многоклеточный и с одной текстурой (как big_lamp),
        // используем spanning-спрайт, иначе — стандартную сетку атласов.
        let spanning = width * height > 1 && tex_count == [1, 1];

        // 1. Маркеры занятости клеток (кроме 0,0, которую занимает сам курсор).
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
            }
        }

        // 2. Визуальный «призрак» объекта.
        if spanning {
            let cx = cursor_x + (width as f32 - 1.0) / 2.0;
            let cy = cursor_y + (height as f32 - 1.0) / 2.0;
            let render_path = format!("{}@{}x{}", tex_path, width, height);
            let entity = crate::ecs::factory::create_sprite(
                &mut self.world, cx, cy, Z_CURSOR,
                &render_path, [0, 0], [1, 1], 1.0, ghost_alpha,
            );
            self.cursor_preview.push(entity);
        } else {
            for i in 0..width {
                for j in 0..height {
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
    }

    // Удаляет все сущности предпросмотра и сбрасывает список.
    pub fn clear_cursor_preview(&mut self) {
        self.delete_entities(&self.cursor_preview);
        self.cursor_preview.clear();
    }
}
