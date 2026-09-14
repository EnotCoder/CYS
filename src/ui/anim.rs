// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  anim.rs — плавные open/close анимации окна (панели).
//  Открытие: «поп» (масштаб 0.6→1.0, easeOutBack). Закрытие: плавное
//  сжатие и затухание всех сущностей панели с последующим удалением.
// ========================================================================

use specs::Entity;
use specs::WorldExt;
use crate::EcsAdapter;
use crate::ecs::components::SpriteComponent;

pub const PANEL_OPEN_DURATION: f64 = 0.28;
pub const PANEL_CLOSE_DURATION: f64 = 0.22;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnimPhase {
    Idle,
    Opening,
    Closing,
}

/// Событие завершения тика анимации.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnimEvent {
    None,
    Opened,
    Closed,
}

/// Общий аниматор панели: держит список сущностей, которые анимирует,
/// и сам удаляет их в конце анимации закрытия.
pub struct PanelAnim {
    phase: AnimPhase,
    entities: Vec<Entity>,
    base_alphas: Vec<f32>,
    /// true, если сущность — текстовый спрайт (`__text__...`, виртуальная
    /// текстура). Такие нельзя масштабировать: кэш спрайтов рендера хранит
    /// растровую текстуру только под scale=1.0, а файла на диске нет.
    is_text: Vec<bool>,
    timer: f64,
    duration: f64,
}

impl PanelAnim {
    pub fn new() -> Self {
        Self {
            phase: AnimPhase::Idle,
            entities: Vec::new(),
            base_alphas: Vec::new(),
            is_text: Vec::new(),
            timer: 0.0,
            duration: PANEL_OPEN_DURATION,
        }
    }

    pub fn phase(&self) -> AnimPhase {
        self.phase
    }

    pub fn is_active(&self) -> bool {
        self.phase != AnimPhase::Idle
    }

    pub fn is_closing(&self) -> bool {
        self.phase == AnimPhase::Closing
    }

    /// Стартует анимацию появления «попа» для набора сущностей.
    pub fn start_open(&mut self, ecs: &mut EcsAdapter, entities: Vec<Entity>) {
        self.reset();
        self.phase = AnimPhase::Opening;
        self.entities = entities;
        self.duration = PANEL_OPEN_DURATION;
        self.capture(ecs);
        let mut sprites = ecs.world.write_storage::<SpriteComponent>();
        for (i, &ent) in self.entities.iter().enumerate() {
            if let Some(sp) = sprites.get_mut(ent) {
                if !self.is_text[i] {
                    sp.scale = 0.6;
                } else {
                    sp.alpha = 0.0;
                }
            }
        }
    }

    /// Стартует анимацию закрытия: сжатие + затухание, затем удаление.
    pub fn start_close(&mut self, ecs: &mut EcsAdapter, entities: Vec<Entity>) {
        self.reset();
        self.phase = AnimPhase::Closing;
        self.entities = entities;
        self.duration = PANEL_CLOSE_DURATION;
        self.capture(ecs);
    }

    /// Мгновенно завершает незавершённое закрытие (удаляет сущности).
    /// Открытие не удаляет сущности — только сбрасывает состояние.
    pub fn cancel(&mut self, ecs: &mut EcsAdapter) {
        if self.phase == AnimPhase::Closing {
            let ents = std::mem::take(&mut self.entities);
            ecs.delete_entities(&ents);
        }
        self.reset();
    }

    /// Прогоняет анимацию на dt секунд и возвращает событие завершения.
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64) -> AnimEvent {
        if self.phase == AnimPhase::Idle {
            return AnimEvent::None;
        }
        self.timer += dt;
        let t = (self.timer / self.duration).clamp(0.0, 1.0) as f32;
        {
            let mut sprites = ecs.world.write_storage::<SpriteComponent>();
            match self.phase {
                AnimPhase::Opening => {
                    // Поп-масштаб для обычных текстур, плавное появление для текста.
                    let e = crate::core::util::ease_out_back(t);
                    let s = 0.6 + 0.4 * e;
                    let a = t * t * (3.0 - 2.0 * t);
                    for (i, &ent) in self.entities.iter().enumerate() {
                        if let Some(sp) = sprites.get_mut(ent) {
                            if self.is_text[i] {
                                sp.alpha = self.base_alphas[i] * a;
                            } else {
                                sp.scale = s;
                            }
                        }
                    }
                }
                AnimPhase::Closing => {
                    let e = t * t;
                    let s = 1.0 - 0.3 * e;
                    for (i, &ent) in self.entities.iter().enumerate() {
                        if let Some(sp) = sprites.get_mut(ent) {
                            sp.alpha = self.base_alphas[i] * (1.0 - e);
                            if !self.is_text[i] {
                                sp.scale = s;
                            }
                        }
                    }
                }
                AnimPhase::Idle => {}
            }
        }
        if t >= 1.0 {
            if self.phase == AnimPhase::Closing {
                let ents = std::mem::take(&mut self.entities);
                self.phase = AnimPhase::Idle;
                ecs.delete_entities(&ents);
                return AnimEvent::Closed;
            }
            self.phase = AnimPhase::Idle;
            self.entities.clear();
            return AnimEvent::Opened;
        }
        AnimEvent::None
    }

    fn capture(&mut self, ecs: &mut EcsAdapter) {
        let sprites = ecs.world.read_storage::<SpriteComponent>();
        self.base_alphas = Vec::with_capacity(self.entities.len());
        self.is_text = Vec::with_capacity(self.entities.len());
        for &ent in &self.entities {
            let sp = sprites.get(ent);
            self.base_alphas.push(sp.map_or(1.0, |s| s.alpha));
            self.is_text.push(sp.is_some_and(|s| s.texture_path.starts_with("__text__")));
        }
    }

    fn reset(&mut self) {
        self.phase = AnimPhase::Idle;
        self.entities.clear();
        self.base_alphas.clear();
        self.is_text.clear();
        self.timer = 0.0;
    }
}