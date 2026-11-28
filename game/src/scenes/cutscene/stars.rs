// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  stars — процедурное звёздное небо кат-сцены.
//
//  Данные генерируются один раз и не меняются до конца приложения. Каждый
//  кадр обновляются только экранные координаты и квантованная альфа.
// ========================================================================

use bytemuck::{Pod, Zeroable};

const SEED: u64 = 0x5EED_1234;

#[derive(Clone, Copy, Debug)]
pub struct Star {
    pub position: [f32; 2],
    pub size: f32,
    pub palette: usize,
    pub phase: f32,
    pub twinkle_speed: f32,
    pub base_alpha: f32,
    pub parallax: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct StarInstance {
    pub center_px: [f32; 2],
    pub size_px: f32,
    pub _pad: f32,
    pub color: [f32; 4],
}

const PALETTE: [[f32; 3]; 4] = [
    [1.0, 1.0, 1.0],
    [195.0 / 255.0, 220.0 / 255.0, 1.0],
    [1.0, 236.0 / 255.0, 205.0 / 255.0],
    [160.0 / 255.0, 185.0 / 255.0, 1.0],
];

const LAYERS: [(usize, f32, (f32, f32), f32); 3] = [
    (170, 0.025, (0.8, 1.3), 160.0),
    (90, 0.060, (1.2, 1.9), 205.0),
    (36, 0.120, (1.8, 2.8), 255.0),
];

fn rand01(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f32) / (1u64 << 53) as f32
}

pub fn generate_stars() -> Vec<Star> {
    let mut seed = SEED;
    let mut stars = Vec::with_capacity(296);
    for &(count, parallax, size_range, base_alpha) in &LAYERS {
        for _ in 0..count {
            stars.push(Star {
                position: [rand01(&mut seed), rand01(&mut seed)],
                size: size_range.0 + rand01(&mut seed) * (size_range.1 - size_range.0),
                palette: (rand01(&mut seed) * PALETTE.len() as f32) as usize % PALETTE.len(),
                phase: rand01(&mut seed) * std::f32::consts::TAU,
                twinkle_speed: 0.6 + rand01(&mut seed) * 1.6,
                base_alpha,
                parallax,
            });
        }
    }
    stars
}

/// Альфа квантуется в четыре уровня, как в исходной реализации; множитель
/// появления применяется ПОСЛЕ квантования, чтобы звёзды могли исчезнуть.
pub fn star_alpha(star: &Star, time: f32, visibility: f32) -> f32 {
    let raw = (star.base_alpha * (0.70 + 0.30 * (time * star.twinkle_speed + star.phase).sin()))
        .clamp(0.0, 255.0);
    let level = ((raw / 64.0) as usize).min(3);
    [64.0, 128.0, 192.0, 255.0][level] / 255.0 * visibility.clamp(0.0, 1.0)
}

/// `wrap` включён в спокойных состояниях: координаты оборачиваются по краям.
/// Во время рывка обёртка выключена, и звёзды уходят за экран без «телепорта».
pub fn build_instances(
    stars: &[Star],
    viewport: (f32, f32),
    time: f32,
    visibility: f32,
    camera: f32,
    wrap: bool,
) -> Vec<StarInstance> {
    let dpi_scale = (viewport.1 / 720.0).clamp(0.6, 2.0);
    stars
        .iter()
        .map(|star| {
            let mut x = star.position[0] * viewport.0 - camera * star.parallax;
            let mut y = star.position[1] * viewport.1 - camera * star.parallax * 0.35;
            if wrap {
                x = x.rem_euclid(viewport.0);
                y = y.rem_euclid(viewport.1);
            }
            let a = star_alpha(star, time, visibility);
            let rgb = PALETTE[star.palette.min(PALETTE.len() - 1)];
            StarInstance {
                center_px: [x, y],
                size_px: star.size * dpi_scale,
                _pad: 0.0,
                color: [rgb[0], rgb[1], rgb[2], a],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_has_three_layers() {
        let a = generate_stars();
        let b = generate_stars();
        assert_eq!(a.len(), 296);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.position, y.position);
            assert_eq!(x.size, y.size);
            assert_eq!(x.palette, y.palette);
            assert_eq!(x.phase, y.phase);
            assert_eq!(x.twinkle_speed, y.twinkle_speed);
        }
        assert_eq!(a.iter().filter(|s| s.parallax == 0.025).count(), 170);
        assert_eq!(a.iter().filter(|s| s.parallax == 0.060).count(), 90);
        assert_eq!(a.iter().filter(|s| s.parallax == 0.120).count(), 36);
        assert!(a
            .iter()
            .all(|s| s.position[0] >= 0.0 && s.position[0] < 1.0));
        assert!(a
            .iter()
            .all(|s| s.position[1] >= 0.0 && s.position[1] < 1.0));
    }

    #[test]
    fn alpha_quantization_and_visibility_order() {
        let star = Star {
            position: [0.0, 0.0],
            size: 1.0,
            palette: 0,
            phase: 0.0,
            twinkle_speed: 1.0,
            base_alpha: 255.0,
            parallax: 0.0,
        };
        for time in [0.0, 0.3, 1.1, 2.7] {
            let a = star_alpha(&star, time, 1.0);
            assert!([64.0, 128.0, 192.0, 255.0]
                .iter()
                .any(|level| (a - level / 255.0).abs() < 1e-6));
        }
        let base = star_alpha(&star, 0.4, 1.0);
        assert!((star_alpha(&star, 0.4, 0.5) - base * 0.5).abs() < 1e-6);
        assert_eq!(star_alpha(&star, 0.4, 0.0), 0.0);
    }

    #[test]
    fn instances_use_quantized_alpha_and_wrap_only_when_needed() {
        let stars = generate_stars();
        let view = (1280.0, 720.0);
        let wrapped = build_instances(&stars, view, 0.0, 1.0, 0.0, true);
        let unwrapped = build_instances(&stars, view, 0.0, 1.0, 9000.0, false);
        assert_eq!(wrapped.len(), 296);
        assert!(wrapped
            .iter()
            .all(|s| s.center_px[0] >= 0.0 && s.center_px[0] < view.0));
        assert!(unwrapped.iter().any(|s| s.center_px[0] < 0.0));
        assert!(wrapped.iter().any(|s| s.color[3] > 0.0));
        assert!(wrapped.iter().all(|s| s.color[3] <= 1.0));
    }
}
