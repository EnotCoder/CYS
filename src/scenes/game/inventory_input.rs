// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  inventory_input.rs — обработка ввода инвентаря и хотбара
// ========================================================================
//  Обрабатывает клики по кнопке инвентаря (E), переключение табов,
//  клики по сетке предметов (перенос в хотбар) и выбор активного слота.
// ========================================================================

use winit::keyboard::KeyCode;
use crate::core::constants::*;
use crate::EcsAdapter;
use crate::ui::system::ndc_to_ui;
use crate::input::platform::InputSource;
use super::GameScene;

impl GameScene {
    /// Обработка кликов по инвентарю: таб, сетка предметов, слоты хотбара
    pub fn handle_inventory_input(&mut self, ecs: &mut EcsAdapter, input: &dyn InputSource, window_size: (f32, f32), device: &wgpu::Device, queue: &wgpu::Queue) {
        // Клавиша E — открыть/закрыть инвентарь
        if input.key_pressed(KeyCode::KeyE) {
            if self.inventory.open {
                self.inventory.exit(ecs);
                crate::audio::play("close");
            } else {
                self.inventory.enter(ecs, device, queue);
                crate::audio::play("open");
            }
        }

        let click = input.mouse_pressed(winit::event::MouseButton::Left);
        if !click {
            return;
        }

        let Some((mx, my)) = input.cursor() else { return };
        let (wx, wy) = ndc_to_ui(mx, my, window_size);

        // --- Клик по слотам хотбара ---
        let hotbar_y = SLOT_BAR_Y + self.bottom_dy;
        let on_bar_y = (wy - hotbar_y).abs() < TILE_HALF;
        let col = (wx - HOTBAR_X + TILE_HALF).floor() as i32;
        if on_bar_y && col >= 0 && col < self.slots.len() as i32 {
            if col != self.act_slot {
                self.act_slot = col;
                if let Some(cursor) = self.icons_slot_cursor {
                    ecs.update_transform_position(cursor, HOTBAR_X + col as f32, hotbar_y);
                }
                crate::audio::play("hover");
            }
            return;
        }

        // --- Клик по кнопке инвентаря на хотбаре ---
        if on_bar_y && (wx - INV_BTN_X).abs() < TILE_HALF {
            return;
        }

        // --- Клик по табам инвентаря ---
        if self.inventory.mode {
            let tab_y = INV_TAB_Y + self.bottom_dy;
            let tcol = (wx - SLOT_BAR_X + TILE_HALF).floor() as i32;
            if (wy - tab_y).abs() < TILE_HALF && tcol >= 0 && tcol < TAB_TEX.len() as i32 {
                if tcol != self.inventory.tab {
                    self.inventory.switch_tab(tcol, ecs, device, queue);
                    crate::audio::play("click");
                }
                return;
            }
        }

        // --- Клик по сетке предметов инвентаря ---
        if self.inventory.mode {
            let col = (wx - SLOT_BAR_X + TILE_HALF).floor() as i32;
            let row = (wy - (INVENTORY_BASE_Y + self.bottom_dy) + TILE_HALF).floor() as i32;

            if col >= 0 && col < INVENTORY_COLS && row >= 0 && row < INVENTORY_ROWS {
                if self.inventory.handle_grid_click(col, row) {
                    self.inventory.transfer_to_slot(ecs, self.act_slot as usize, &mut self.slots, &self.slot_entities);
                    crate::audio::play("click");
                }
                return;
            }

            // Клик в любое другое место (вне слотов, вне табов и вне сетки) закрывает инвентарь
            self.inventory.exit(ecs);
            crate::audio::play("close");
        }
    }
}
