// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  inventory_input.rs — обработка ввода инвентаря и хотбара
// ========================================================================
//  Обрабатывает клики по кнопке инвентаря (E), переключение табов,
//  клики по сетке предметов (перенос в хотбар) и выбор активного слота.
// ========================================================================

use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::core::util::ndc_to_world;
use super::GameScene;

impl GameScene {
    /// Обработка кликов по инвентарю: таб, сетка предметов, слоты хотбара
    pub fn handle_inventory_input(&mut self, ecs: &mut EcsAdapter, input: &WinitInputHelper, window_size: (f32, f32)) {
        // Клавиша E — открыть/закрыть инвентарь
        if input.key_pressed(KeyCode::KeyE) {
            if self.inventory.open {
                self.inventory.exit(ecs);
            } else {
                self.inventory.enter(ecs);
            }
            crate::audio::play("click");
        }

        let click = input.mouse_pressed(winit::event::MouseButton::Left);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let (wx, wy) = ndc_to_world(mx, my, window_size, 1.0, 0.0, 0.0);

        // --- Клик по табам ---
        if self.inventory.mode {
            let tcol = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            if (wy - INV_TAB_Y).abs() < TILE_HALF && tcol >= 0 && tcol < TAB_TEX.len() as i32 {
                if tcol != self.inventory.tab {
                    self.inventory.switch_tab(tcol, ecs);
                    crate::audio::play("click");
                }
                return;
            }
        }

        // --- Клик по сетке инвентаря ---
        if self.inventory.mode {
            let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
            let row = (wy - INVENTORY_BASE_Y + TILE_HALF) as i32;
            if self.inventory.handle_grid_click(col, row) {
                // Предмет из инвентаря переносится в выбранный слот хотбара
                self.inventory.transfer_to_slot(ecs, self.act_slot as usize, &mut self.slots, &self.slot_entities);
                crate::audio::play("click");
            }
            return;
        }

        // --- Клик по слотам хотбара ---
        let col = (wx - SLOT_BAR_X + TILE_HALF) as i32;
        if (wy - SLOT_BAR_Y).abs() < TILE_HALF && col >= 0 && col < self.slots.len() as i32 {
            let target = col;
            if target != self.act_slot {
                // Деактивируем старый слот, активируем новый и двигаем рамку выбора
                if let Some(cursor) = self.icons_slot_cursor {
                    let old = self.act_slot as usize;
                    if old < self.slots.len() {
                        self.slots[old].active = false;
                    }
                    self.act_slot = target;
                    self.slots[target as usize].active = true;
                    ecs.update_transform_position(cursor, SLOT_BAR_X + col as f32, SLOT_BAR_Y);
                }
                crate::audio::play("hover");
            }
        }
    }
}
