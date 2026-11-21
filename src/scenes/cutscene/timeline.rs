// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  timeline — независимая от рендера временная шкала вступительной кат-сцены.
// ========================================================================

/// Смысловой план, который сейчас на экране.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Dark,
    StarsReveal,
    StarsHold,
    JumpToSolar,
    SolarHold,
    JumpToContinent,
    ContinentHold,
    JumpToShop,
    ShopHold,
}

/// Источник изображения для fullscreen-плана.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    Dark,
    Stars,
    Solar,
    Continent,
    Shop,
}

pub const TIMELINE: [(Stage, f32); 9] = [
    (Stage::Dark, 0.40),
    (Stage::StarsReveal, 0.60),
    (Stage::StarsHold, 1.20),
    (Stage::JumpToSolar, 0.22),
    (Stage::SolarHold, 1.60),
    (Stage::JumpToContinent, 0.22),
    (Stage::ContinentHold, 1.20),
    (Stage::JumpToShop, 0.22),
    (Stage::ShopHold, 1.60),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneTransform {
    pub center: [f32; 2],
    pub zoom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpParams {
    /// Точка приближения в UV исходного плана.
    pub focus: [f32; 2],
    /// Масштаб, в который уезжает исходный план.
    pub a_end_zoom: f32,
    /// Масштаб прибывающего плана в момент подмены.
    pub b_start_zoom: f32,
    /// Центр прибывающего плана в момент подмены.
    pub b_start_center: [f32; 2],
    /// Сила радиального смаза в UV.
    pub blur: f32,
}

pub fn plane_of(stage: Stage) -> Plane {
    match stage {
        Stage::Dark => Plane::Dark,
        Stage::StarsReveal | Stage::StarsHold => Plane::Stars,
        Stage::JumpToSolar => Plane::Stars,
        Stage::SolarHold => Plane::Solar,
        Stage::JumpToContinent => Plane::Solar,
        Stage::ContinentHold => Plane::Continent,
        Stage::JumpToShop => Plane::Continent,
        Stage::ShopHold => Plane::Shop,
    }
}

pub fn next_plane_of(stage: Stage) -> Option<Plane> {
    match stage {
        Stage::JumpToSolar => Some(Plane::Solar),
        Stage::JumpToContinent => Some(Plane::Continent),
        Stage::JumpToShop => Some(Plane::Shop),
        _ => None,
    }
}

/// Зафиксированный transform плана в состоянии удержания. Рывок начинается
/// именно с него и заканчивается им же — после остановки нет отскока масштаба.
pub fn plane_transform(plane: Plane) -> PlaneTransform {
    match plane {
        Plane::Dark => PlaneTransform {
            center: [0.5, 0.5],
            zoom: 1.0,
        },
        Plane::Stars => PlaneTransform {
            center: [0.5, 0.5],
            zoom: 1.0,
        },
        // Солнце слева, Земля — центр следующего приближения.
        Plane::Solar => PlaneTransform {
            center: [0.62, 0.47],
            zoom: 1.14,
        },
        Plane::Continent => PlaneTransform {
            center: [0.53, 0.51],
            zoom: 1.06,
        },
        Plane::Shop => PlaneTransform {
            center: [0.5, 0.5],
            zoom: 1.0,
        },
    }
}

pub fn jump_params(stage: Stage) -> JumpParams {
    match stage {
        // Звёзды уходят за края: фокус в центре, сильное увеличение.
        Stage::JumpToSolar => JumpParams {
            focus: [0.5, 0.5],
            a_end_zoom: 4.2,
            b_start_zoom: 1.9,
            b_start_center: [0.5, 0.5],
            blur: 0.045,
        },
        // Точка на Земле совпадает с положением Земли в плане Солнечной системы.
        Stage::JumpToContinent => JumpParams {
            focus: [0.64, 0.46],
            a_end_zoom: 8.0,
            b_start_zoom: 2.1,
            b_start_center: [0.53, 0.51],
            blur: 0.055,
        },
        // Точка магазина на континенте совпадает с меткой на карте.
        Stage::JumpToShop => JumpParams {
            focus: [0.56, 0.47],
            a_end_zoom: 7.0,
            b_start_zoom: 1.75,
            b_start_center: [0.5, 0.5],
            blur: 0.050,
        },
        _ => JumpParams {
            focus: [0.5, 0.5],
            a_end_zoom: 1.0,
            b_start_zoom: 1.0,
            b_start_center: [0.5, 0.5],
            blur: 0.0,
        },
    }
}

#[derive(Clone, Debug)]
pub struct IntroClock {
    index: usize,
    elapsed: f32,
}

impl Default for IntroClock {
    fn default() -> Self {
        Self::new()
    }
}

impl IntroClock {
    pub fn new() -> Self {
        Self {
            index: 0,
            elapsed: 0.0,
        }
    }

    /// Прибавляет время кадра, перенося остаток через границы состояний.
    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.elapsed += dt;
        while let Some(&(_, duration)) = TIMELINE.get(self.index) {
            if self.elapsed < duration {
                break;
            }
            self.elapsed -= duration;
            self.index += 1;
        }
    }

    /// `(этап, прогресс 0..1)`; `None` — заставка закончена.
    pub fn current(&self) -> Option<(Stage, f32)> {
        TIMELINE.get(self.index).map(|&(stage, duration)| {
            (
                stage,
                (self.elapsed / duration.max(f32::EPSILON)).clamp(0.0, 1.0),
            )
        })
    }

    pub fn skip(&mut self) {
        self.index = TIMELINE.len();
        self.elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_starts_dark_and_finishes_after_total_duration() {
        let mut clock = IntroClock::new();
        assert_eq!(clock.current(), Some((Stage::Dark, 0.0)));

        let total: f32 = TIMELINE.iter().map(|&(_, d)| d).sum();
        clock.update(total - 0.01);
        assert_eq!(clock.current().map(|(s, _)| s), Some(Stage::ShopHold));
        clock.update(0.02);
        assert_eq!(clock.current(), None);
    }

    #[test]
    fn clock_carries_remainder_across_boundaries() {
        let mut clock = IntroClock::new();
        clock.update(TIMELINE[0].1 + TIMELINE[1].1 * 0.25);
        let (stage, progress) = clock.current().unwrap();
        assert_eq!(stage, Stage::StarsReveal);
        assert!((progress - 0.25).abs() < 1e-5);

        clock.update(10.0);
        assert_eq!(clock.current(), None);
    }

    #[test]
    fn exact_boundary_advances_to_next_stage() {
        let mut clock = IntroClock::new();
        for &(_, duration) in &TIMELINE[..3] {
            clock.update(duration);
        }
        assert_eq!(clock.current(), Some((Stage::JumpToSolar, 0.0)));

        let mut clock = IntroClock::new();
        clock.update(TIMELINE[0].1);
        assert_eq!(clock.current(), Some((Stage::StarsReveal, 0.0)));
    }

    #[test]
    fn clock_ignores_invalid_dt_and_skip_ends_immediately() {
        let mut clock = IntroClock::new();
        clock.update(f32::NAN);
        clock.update(-1.0);
        assert_eq!(clock.current(), Some((Stage::Dark, 0.0)));
        clock.skip();
        assert_eq!(clock.current(), None);
    }

    #[test]
    fn jumps_have_distinct_focus_points_and_keep_hold_transform() {
        for stage in [
            Stage::JumpToSolar,
            Stage::JumpToContinent,
            Stage::JumpToShop,
        ] {
            let p = jump_params(stage);
            assert!(p.a_end_zoom > 1.0);
            assert!(p.b_start_zoom > 1.0);
            assert!(p.blur > 0.0);
            let held = plane_transform(plane_of(stage));
            assert!(p.a_end_zoom > held.zoom);
            let arrived = plane_transform(next_plane_of(stage).unwrap());
            assert!(p.b_start_zoom > arrived.zoom);
        }
        assert_ne!(
            jump_params(Stage::JumpToContinent).focus,
            jump_params(Stage::JumpToShop).focus
        );
    }
}
