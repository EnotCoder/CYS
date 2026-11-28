// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  cutscene — вступительная кат-сцена между логотипом и главным меню.
// ========================================================================

pub mod renderer;
pub mod stars;
pub mod timeline;

use std::time::Instant;

use crate::{
    core::constants::MAX_FRAME_DT,
    scenes::{MenuScene, Scene, SceneAction},
};

use self::{
    renderer::CutsceneRenderer,
    timeline::{IntroClock, Stage},
};

const SKIP_HINT: &str = "Any key to skip";
const SKIP_HINT_FONT_PX: f32 = 14.0;
const SKIP_HINT_COLOR: [u8; 3] = [186, 196, 214];
const SKIP_HINT_DELAY: f32 = 0.45;

pub struct IntroScene {
    clock: IntroClock,
    renderer: Option<CutsceneRenderer>,
    last_stage: Option<Stage>,
    last_frame: Option<Instant>,
    dt: f32,
    total_time: f32,
    skip_ready: bool,
}

impl IntroScene {
    pub fn new() -> Self {
        Self {
            clock: IntroClock::new(),
            renderer: None,
            last_stage: None,
            last_frame: None,
            dt: 0.0,
            total_time: 0.0,
            skip_ready: false,
        }
    }

    fn visuals_for(stage: Stage, progress: f32) -> (f32, f32, bool) {
        match stage {
            Stage::Dark => (0.0, 0.0, true),
            Stage::StarsReveal => {
                let p = progress.clamp(0.0, 1.0);
                (p * p * (3.0 - 2.0 * p), 0.0, true)
            }
            Stage::StarsHold => (1.0, 0.0, true),
            // Движение даёт сам зум композита, звёзды не уезжают за экран —
            // иначе на стыке с планом Солнца они бы резко вернулись назад.
            Stage::JumpToSolar => (1.0, 0.0, true),
            // План Солнца рисует те же звёзды, поэтому они видимы и здесь.
            Stage::SolarHold | Stage::JumpToContinent => (1.0, 0.0, true),
            _ => (0.0, 0.0, true),
        }
    }

    fn play_swish_if_needed(&mut self, stage: Stage) {
        let jump = matches!(
            stage,
            Stage::JumpToSolar | Stage::JumpToContinent | Stage::JumpToShop
        );
        if jump && self.last_stage != Some(stage) {
            crate::audio::play("swish");
        }
        self.last_stage = Some(stage);
    }

    fn skip(&mut self) -> SceneAction {
        self.clock.skip();
        SceneAction::Switch("menu".to_string())
    }
}

impl Default for IntroScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for IntroScene {
    fn on_enter(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        _text_renderer: &mut crate::ui::text_renderer::TextRenderer,
    ) {
        let renderer = self.renderer.take();
        *self = Self::new();
        self.renderer = renderer;
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.reset_shop();
        }

        // Кадр магазина берём из той же карты и декора, что и фоне меню.
        crate::data::map::load_map_to_ecs(ecs);
        MenuScene::place_decor(ecs);
    }

    fn update(
        &mut self,
        _ecs: &mut crate::EcsAdapter,
        input: &dyn crate::input::platform::InputSource,
        _window_size: (f32, f32),
        _text_renderer: &mut crate::ui::text_renderer::TextRenderer,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> SceneAction {
        let dt = match self.last_frame {
            Some(t0) => t0.elapsed().as_secs_f32(),
            None => 1.0 / 60.0,
        }
        .clamp(0.0, MAX_FRAME_DT as f32);
        self.last_frame = Some(Instant::now());
        self.dt = dt;
        self.total_time += dt;
        self.clock.update(dt);

        if let Some((stage, _)) = self.clock.current() {
            self.play_swish_if_needed(stage);
        }

        if input.any_pressed() {
            if self.skip_ready {
                return self.skip();
            }
            self.skip_ready = true;
        } else {
            self.skip_ready = true;
        }

        if self.clock.current().is_none() {
            return SceneAction::Switch("menu".to_string());
        }
        SceneAction::None
    }

    fn sprites(
        &self,
        _ecs: &crate::EcsAdapter,
        _visible_bounds: Option<(f32, f32, f32, f32)>,
    ) -> (
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
        Vec<crate::SpriteRenderData>,
    ) {
        Default::default()
    }

    fn map_size(&self) -> f32 {
        1.0
    }

    fn camera_offset(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn uses_custom_render(&self) -> bool {
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn render_custom(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        wgpu_app: &crate::core::WgpuApp,
        surface: &wgpu::Surface,
        window_size: (f32, f32),
        text_renderer: &mut crate::ui::text_renderer::TextRenderer,
    ) -> bool {
        if window_size.0 <= 0.0 || window_size.1 <= 0.0 {
            return false;
        }
        let size = (window_size.0 as u32, window_size.1 as u32);

        if self.renderer.is_none() {
            let mut renderer = CutsceneRenderer::new(
                &wgpu_app.device,
                &wgpu_app.queue,
                wgpu_app.surface_format,
                size,
            );
            let (pixels, width, height) =
                text_renderer.raster_rgba(SKIP_HINT, SKIP_HINT_FONT_PX, 0.0, SKIP_HINT_COLOR);
            renderer.set_hint(&wgpu_app.device, &wgpu_app.queue, pixels, width, height);
            self.renderer = Some(renderer);
        }
        let renderer = self.renderer.as_mut().unwrap();
        renderer.resize(&wgpu_app.device, size);
        if !renderer.shop_captured() {
            renderer.capture_shop(ecs, wgpu_app, size);
        }

        let Some((stage, progress)) = self.clock.current() else {
            return false;
        };
        let (visibility, camera, wrap) = Self::visuals_for(stage, progress);
        renderer.prepare(
            &wgpu_app.queue,
            window_size,
            self.total_time,
            visibility,
            camera,
            wrap,
        );
        renderer.render_frame(
            surface,
            &wgpu_app.device,
            &wgpu_app.queue,
            stage,
            progress,
            self.total_time,
            1.0,
            self.total_time >= SKIP_HINT_DELAY,
        )
    }
}
