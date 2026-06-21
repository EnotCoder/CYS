// ========================================================================
//  FPS счётчик — замеряет кадры в секунду
// ========================================================================

use std::time::Instant;

pub struct FpsCounter {
    last_time: Instant,
    frame_count: u32,
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
        if elapsed >= std::time::Duration::from_secs(1) {
            self.fps = (self.frame_count as f64 / elapsed.as_secs_f64()).round() as u32;
            self.frame_count = 0;
            self.last_time = Instant::now();
        }
        self.fps
    }
}
