use crate::script::config::BalanceConfig;

// ========================================================================
//  DayNightCycle — цикл день/ночь
// ========================================================================
//  Считает прошедшее время и возвращает коэффициент затенения сцены (0 — день,
//  1 — ночь) с плавными переходами на рассвете и закате. Длительности фаз
//  берутся из BalanceConfig. Этот коэффициент подаётся в шейдер в
//  GameScene::night_factor() и затемняет картинку.

pub struct DayNightCycle {
    elapsed: f64,
}

impl DayNightCycle {
    pub fn new() -> Self {
        Self { elapsed: 0.0 }
    }

    /// Сброс цикла (новый заход в сцену)
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    /// Накапливает игровое время
    pub fn tick(&mut self, dt: f64) {
        self.elapsed += dt;
    }

    /// Коэффициент затенения 0..1 в текущий момент времени.
    /// В течение дня — 0, ночью — 1, между ними — линейный плавный переход.
    pub fn factor(&self, config: &BalanceConfig) -> f32 {
        let cycle_sec = self.elapsed % config.day_cycle_secs;
        let fade = config.fade_secs;
        if cycle_sec < config.day_secs {
            0.0
        } else if cycle_sec < config.night_start_secs {
            // Закат: плавно светлеет к полной ночи
            ((cycle_sec - config.day_secs) / fade) as f32
        } else if cycle_sec < config.night_secs {
            1.0
        } else {
            // Рассвет: плавно возвращается к дню
            (1.0 - (cycle_sec - config.night_secs) / fade) as f32
        }
    }

    /// Игровое время в формате "T:  ЧЧ:ММ" (24 часа = полный игровой цикл)
    pub fn time_string(&self, config: &BalanceConfig) -> String {
        let cycle = config.day_cycle_secs;
        let total_sec = self.elapsed % cycle;
        let hours_f = total_sec / cycle * 24.0;
        let hours = (hours_f as i32) % 24;
        let minutes = ((hours_f - hours_f.floor()) * 60.0) as i32;
        format!("T:  {:02}:{:02}", hours, minutes)
    }
}