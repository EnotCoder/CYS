use crate::script::config::BalanceConfig;

pub struct DayNightCycle {
    elapsed: f64,
}

impl DayNightCycle {
    pub fn new() -> Self {
        Self { elapsed: 0.0 }
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn tick(&mut self, dt: f64) {
        self.elapsed += dt;
    }

    pub fn factor(&self, config: &BalanceConfig) -> f32 {
        let cycle_sec = self.elapsed % config.day_cycle_secs;
        let fade = config.fade_secs;
        if cycle_sec < config.day_secs {
            0.0
        } else if cycle_sec < config.night_start_secs {
            ((cycle_sec - config.day_secs) / fade) as f32
        } else if cycle_sec < config.night_secs {
            1.0
        } else {
            (1.0 - (cycle_sec - config.night_secs) / fade) as f32
        }
    }

    pub fn time_string(&self, config: &BalanceConfig) -> String {
        let cycle = config.day_cycle_secs;
        let total_sec = self.elapsed % cycle;
        let hours_f = total_sec / cycle * 24.0;
        let hours = (hours_f as i32) % 24;
        let minutes = ((hours_f - hours_f.floor()) * 60.0) as i32;
        format!("T:  {:02}:{:02}", hours, minutes)
    }
}