// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  frame_profiler: почастовые замеры кадра по этапам.
//  Включается переменной окружения CYS_PROFILE=1, раз в секунду печатает
//  среднее и максимум в stderr и не меняет игровое состояние.
// ========================================================================

use std::time::{Duration, Instant};

const STAGE_NAMES: [&str; 5] = ["update", "sprites", "uniforms", "render_cache", "render_encode"];

#[derive(Clone, Copy)]
pub enum StageId {
    Update = 0,
    Sprites = 1,
    Uniforms = 2,
    RenderCache = 3,
    RenderEncode = 4,
}

pub struct FrameProfiler {
    enabled: bool,
    frames: u32,
    last_report: Instant,
    sums: [f64; 5],
    maxs: [f64; 5],
}

impl FrameProfiler {
    pub fn new() -> Self {
        let enabled = std::env::var("CYS_PROFILE")
            .map(|value| value == "1")
            .unwrap_or(false);
        Self {
            enabled,
            frames: 0,
            last_report: Instant::now(),
            sums: [0.0; 5],
            maxs: [0.0; 5],
        }
    }

    pub fn stamp(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }

    pub fn record(&mut self, stage: StageId, start: Option<Instant>) {
        let Some(start) = start else {
            return;
        };
        self.record_duration(stage, start.elapsed());
    }

    pub fn record_duration(&mut self, stage: StageId, elapsed: Duration) {
        let ms = elapsed.as_secs_f64() * 1000.0;
        let index = stage as usize;
        self.sums[index] += ms;
        self.maxs[index] = self.maxs[index].max(ms);
    }

    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_report);
        if self.frames == 0 || elapsed < Duration::from_secs(1) {
            return;
        }
        let frames = self.frames as f64;
        let mut line = format!(
            "CYS_PROFILE frames={} fps={:.1}",
            self.frames,
            frames / elapsed.as_secs_f64()
        );
        for (i, name) in STAGE_NAMES.iter().enumerate() {
            line.push_str(&format!(
                " | {}: {:.2}/{:.2} ms",
                name,
                self.sums[i] / frames,
                self.maxs[i]
            ));
        }
        eprintln!("{line}");
        self.frames = 0;
        self.last_report = now;
        self.sums = [0.0; 5];
        self.maxs = [0.0; 5];
    }

    pub fn end_frame(&mut self) {
        if self.enabled {
            self.frames += 1;
        }
    }
}
