// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  FPS счётчик — замеряет кадры в секунду
// ========================================================================

use std::time::Instant;

pub struct FpsCounter {
    // Момент начала текущего секундного окна замера.
    last_time: Instant,
    // Число кадров, показанных за текущее окно.
    frame_count: u32,
    // Текущее значение FPS (обновляется раз в секунду).
    fps: u32,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }

    /// Звать каждый кадр. Возвращает текущий FPS (обновляется раз в секунду).
    pub fn tick(&mut self) -> u32 {
        self.frame_count += 1;
        let elapsed = self.last_time.elapsed();
        // Раз в секунду пересчитываем FPS и открываем новое окно замера.
        if elapsed >= std::time::Duration::from_secs(1) {
            self.fps = (self.frame_count as f64 / elapsed.as_secs_f64()).round() as u32;
            self.frame_count = 0;
            self.last_time = Instant::now();
        }
        self.fps
    }
}
