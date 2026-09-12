// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// Погодные эффекты: зимой идёт снег (мелкие белые квадраты), осенью — дождь
// (вытянутые вниз прямоугольники). Частицы размещаются в мировых координатах
// внутри видимой области, падают сверху вниз и возвращаются наверх за её
// пределами. Текстуры частиц генерируются в памяти (не нужны файлы-ассеты).

use crate::core::constants::Z_WEATHER;
use crate::core::util;
use crate::ecs::components::{Season, Transform};
use crate::ecs::factory::create_sprite;
use specs::WorldExt;
use crate::EcsAdapter;

const SNOW_COUNT: usize = 90;
const RAIN_COUNT: usize = 80;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    Snow,
    Rain,
}

struct Particle {
    entity: specs::Entity,
    x: f32,
    y: f32,
    speed: f32,
    drift: f32,
}

pub struct WeatherFx {
    // Активный эффект (None — лето/весна без осадков)
    effect: Option<Effect>,
    particles: Vec<Particle>,
    // Таймер случайного грома во время дождя (секунды до следующего прогресса)
    thunder_timer: f32,
    // Семя простого LCG-генератора (без внешних зависимостей на rand)
    seed: u64,
}

impl WeatherFx {
    pub fn new() -> Self {
        Self { effect: None, particles: Vec::new(), thunder_timer: 0.0, seed: 0x2545_4914_F6CD_DD1D }
    }

    // Сброс при входе в сцену: мир и кэши уже очищены снаружи.
    pub fn reset(&mut self) {
        self.effect = None;
        self.particles.clear();
        self.thunder_timer = 0.0;
    }

    fn effect_for(season: Season) -> Option<Effect> {
        match season {
            Season::Winter => Some(Effect::Snow),
            Season::Autumn => Some(Effect::Rain),
            _ => None,
        }
    }

    fn sprite_path(effect: Effect) -> &'static str {
        match effect {
            Effect::Snow => "weather/snow",
            Effect::Rain => "weather/rain",
        }
    }

    fn sprite_key(effect: Effect) -> u64 {
        util::sprite_cache_key("transparent", Self::sprite_path(effect), [0, 0], [1, 1], 1.0)
    }

    // Ежекадровое обновление погоды: применяет смену сезона, двигает частицы
    // вниз и возвращает выпавшие за видимую область наверх.
    pub fn tick(
        &mut self,
        ecs: &mut EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        season: Season,
        dt: f64,
        bounds: (f32, f32, f32, f32),
    ) {
        let want = Self::effect_for(season);
        let alive = self.particles.iter().all(|p| {
            ecs.world.read_storage::<Transform>().get(p.entity).is_some()
        });
        // Пересоздаём поле, если сезон сменился или сущности исчезли (смена уровня).
        let needs_respawn = want != self.effect || (want.is_some() && (!alive || self.particles.is_empty()));
        if needs_respawn {
            self.teardown(ecs);
            self.effect = want;
            // Включаем/выключаем звук погоды при смене сезона.
            match want {
                Some(Effect::Snow) => crate::audio::play_ambient("snow"),
                Some(Effect::Rain) => crate::audio::play_ambient("rain"),
                None => crate::audio::stop_ambient(),
            }
            if let Some(effect) = want {
                self.spawn(effect, ecs, device, queue, bounds);
            }
        }
        if want.is_none() {
            return;
        }

        let dt32 = dt as f32;
        let (l, r, b, t) = bounds;
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.y -= p.speed * dt32;
            p.x += p.drift * dt32;
            if p.y < b - 0.3 {
                let r1 = rand01(&mut self.seed);
                let r2 = rand01(&mut self.seed);
                p.y = t + r1 * 0.4;
                p.x = l + r2 * (r - l);
            }
            // При движении камеры возвращаем частицы в видимую область.
            if p.x < l - 0.4 || p.x > r + 0.4 {
                let r = rand01(&mut self.seed);
                p.x = l + r * (r - l);
            }
            ecs.update_transform_position(p.entity, p.x, p.y);
        }

        // Редкий гром во время дождя: ждём случайный интервал и играем звук.
        if self.effect == Some(Effect::Rain) {
            self.thunder_timer -= dt32;
            if self.thunder_timer <= 0.0 {
                crate::audio::play("thunder");
                self.thunder_timer = 12.0 + rand01(&mut self.seed) * 25.0;
            }
        }
    }

    fn teardown(&mut self, ecs: &mut EcsAdapter) {
        if let Some(effect) = self.effect {
            ecs.sprite_cache.remove(&Self::sprite_key(effect));
        }
        for p in self.particles.drain(..) {
            ecs.delete_entity(p.entity);
        }
    }

    fn spawn(
        &mut self,
        effect: Effect,
        ecs: &mut EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: (f32, f32, f32, f32),
    ) {
        let (l, r, b, t) = bounds;
        let (world_w, world_h, count, alpha, min_speed, max_speed, drift) = match effect {
            Effect::Snow => (0.10f32, 0.10f32, SNOW_COUNT, 0.9f32, 0.6f32, 1.4f32, 0.06f32),
            Effect::Rain => (0.05f32, 0.30f32, RAIN_COUNT, 0.75f32, 3.2f32, 4.8f32, 0.0f32),
        };
        let path = Self::sprite_path(effect);

        // Частицы — простые пиксельные текстуры: снег — белый квадрат,
        // дождь — вытянутая вниз полоска.
        let (tw, th, rgba): (u32, u32, Vec<u8>) = match effect {
            Effect::Snow => (4, 4, vec![255u8; 4 * 4 * 4]),
            Effect::Rain => {
                let px = [185u8, 210u8, 255u8, 255u8];
                let rgba: Vec<u8> = px.iter().copied().cycle().take(2 * 16 * 4).collect();
                (2, 16, rgba)
            }
        };
        let tex = crate::Texture::from_rgba(device, queue, &rgba, tw, th, "weather_particle");
        let sprite = crate::Sprite::from_texture(device, &tex, path, world_w, world_h);
        ecs.sprite_cache.insert(Self::sprite_key(effect), sprite);

        for _ in 0..count {
            let x = l + rand01(&mut self.seed) * (r - l);
            let y = t - rand01(&mut self.seed) * (t - b);
            let speed = min_speed + (max_speed - min_speed) * rand01(&mut self.seed);
            let drift = drift * (rand01(&mut self.seed) - 0.5) * 2.0;
            let entity = create_sprite(
                &mut ecs.world, x, y, Z_WEATHER,
                path, [0, 0], [1, 1], 1.0, alpha,
            );
            self.particles.push(Particle { entity, x, y, speed, drift });
        }
    }
}

// Равномерное число в [0,1): простая замена rand без внешних крейтов.
fn rand01(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f32) / (1u64 << 53) as f32
}