// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  CYS — Create your Shop: точка входа приложения
//  Корень крейта (src/main/main.rs, см. [[bin]] в Cargo.toml):
//  объявляет 12 папок src/, создаёт окно, GPU-контекст и запускает
//  главный цикл событий.
// ========================================================================

#[path = "../audio/mod.rs"]
mod audio;
#[path = "../core/mod.rs"]
mod core;
#[path = "../data/mod.rs"]
mod data;
#[path = "../ecs/mod.rs"]
mod ecs;
#[path = "../input/mod.rs"]
mod input;
#[path = "../npc/mod.rs"]
mod npc;
#[path = "../scenes/mod.rs"]
mod scenes;
#[path = "../scripts/mod.rs"]
mod scripts;
#[path = "../ui/mod.rs"]
mod ui;
#[path = "../tests/mod.rs"]
#[cfg(test)]
mod tests;

// Реэкспорты, к которым остальной код обращается как crate::* (EcsAdapter, Vertex, ...)
use crate::core::constants::*;
use crate::core::*;
use crate::ecs::*;
use crate::input::platform::{DesktopInput, InputSource};
use crate::ui::fps::FpsCounter;

use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;

struct App {
    // Окно используется в течение всей жизни приложения (leak в 'static)
    window: Option<&'static Window>,
    // GPU-контекст: устройство, очередь, пайплайны, буферы
    wgpu_app: Option<WgpuApp>,
    // Поверхность вывода (привязана к окну)
    surface: Option<wgpu::Surface<'static>>,
    // Рендер текста (меню, подсказки, FPS)
    text_renderer: crate::ui::text_renderer::TextRenderer,
    // Менеджер сцен: хранит ECS, переключение между меню и игрой
    scene_manager: crate::scenes::SceneManager,
    // Накопитель ввода: десктоп (мышь/клавиши) или тач (Android) через единый трейт
    input: Box<dyn InputSource>,
    fps_counter: FpsCounter,
    // Флаг выхода из приложения
    quit_requested: bool,
}

impl App {
    // Единый конструктор для десктопа и Android: источник ввода передаётся
    // снаружи (DesktopInput для ПК, TouchInput для мобильных).
    pub fn new(input: Box<dyn InputSource>) -> Self {
        crate::audio::init();
        let config = crate::scripts::config::BalanceConfig::load();
        let font_path = config.font_path.clone();
        Self {
            window: None,
            wgpu_app: None,
            surface: None,
            text_renderer: crate::ui::text_renderer::TextRenderer::new(&font_path),
            scene_manager: crate::scenes::SceneManager::new(),
            input,
            fps_counter: FpsCounter::new(),
            quit_requested: false,
        }
    }

    // Один кадр: обновить сцену -> применить действие сцены -> отрисовать
    fn render(&mut self) {
        let Some(surface) = &self.surface else { return };
        let Some(ref window) = self.window else { return };

        let window_size = (
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );

        // Шаг 1: update сцены — временно берём wgpu_app immutably
        // (внутри обрабатываются ввод, логика игры и возвращается действие сцены)
            let action = {
            let Some(ref wgpu_app) = self.wgpu_app else { return };
            let scene = self.scene_manager.scenes.get_mut(&self.scene_manager.current).unwrap();
            scene.update(
                &mut self.scene_manager.ecs,
                &*self.input,
                window_size,
                &mut self.text_renderer,
                &wgpu_app.device,
                &wgpu_app.queue,
            )
        };

        // Шаг 2: обрабатываем action — borrow wgpu_app освобождён
        match action {
            crate::scenes::SceneAction::Switch(name) => {
                // Запрос на смену сцены (например, из меню в игру)
                self.scene_manager.switch_to(&name, &mut self.text_renderer);
            }
            crate::scenes::SceneAction::Quit => {
                self.quit_requested = true;
            }
            crate::scenes::SceneAction::VsyncToggle(enabled) => {
                // Переключение вертикальной синхронизации на лету
                if let Some(ref mut wgpu_app) = self.wgpu_app {
                    if let Some(ref surface) = self.surface {
                        let desired = if enabled { wgpu::PresentMode::Fifo } else { wgpu::PresentMode::Mailbox };
                        let mode = if wgpu_app.surface_caps.present_modes.contains(&desired) {
                            desired
                        } else {
                            wgpu_app.surface_caps.present_modes[0]
                        };
                        wgpu_app.config.present_mode = mode;
                        surface.configure(&wgpu_app.device, &wgpu_app.config);
                    }
                }
            }
            crate::scenes::SceneAction::None => {}
        }

        // Шаг 3: рендер — снова берём wgpu_app immutably
        let Some(ref wgpu_app) = self.wgpu_app else { return };

        let fps = self.fps_counter.tick();
        self.scene_manager.update_fps(fps, &mut self.text_renderer, &wgpu_app.device, &wgpu_app.queue);

        let ms = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().map_size();
        let (cam_x, cam_y) = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().camera_offset();
        let nf = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().night_factor();
        let aspect = window_size.0 / window_size.1;

        // Видимая область мира (для culling спрайтов за кадром)
        let vis_w = 2.0 * aspect / (SHADER_SCALE * ms);
        let vis_h = 2.0 / (SHADER_SCALE * ms);
        let bounds = Some((cam_x - vis_w/2.0, cam_x + vis_w/2.0, cam_y - vis_h/2.0, cam_y + vis_h/2.0));
        // Сбор спрайтов по слоям (z-order: карта, ковры, декорации, персонажи, курсор, UI)
        let scene = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap();
        let (map_sprites, carpet_sprites, light_sprites, decor_sprites, npc_sprites, cursor_sprites, ui_sprites) =
            scene.sprites(&self.scene_manager.ecs, bounds);
        let lights = scene.lights(&self.scene_manager.ecs);

        // Обновляем количество источников света в Size uniform
        let light_count = lights.len() as u32;

        // Мировые uniform'ы: карта, камера и освещение для мировых шейдеров
        let size_data = Size { 
            map_size: ms, 
            aspect, 
            offset_x: cam_x, 
            offset_y: cam_y, 
            night_factor: nf,
            light_count,
            _padding: [0.0; 2],
        };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));

        // Uniform'ы для UI-слоя (UI рендерится без камеры/ночи)
        let ui_uniforms = UiUniforms { 
            size: 1.0,
            aspect, 
            _padding1: [0.0; 2],
            night_factor: 0.0,
            light_count: 0,
            _padding2: [0.0; 2],
        };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

        render(
            surface,
            &wgpu_app.device,
            &wgpu_app.queue,
            &wgpu_app.render_pipeline,
            &wgpu_app.transparent_pipeline,
            &wgpu_app.depth_buffer.view,
            &map_sprites,
            &carpet_sprites,
            &light_sprites,
            &decor_sprites,
            &npc_sprites,
            &cursor_sprites,
            &ui_sprites,
            &wgpu_app.size_bind_group,
            &wgpu_app.ui_bind_group,
            &mut self.scene_manager.ecs.sprite_cache,
            &mut self.scene_manager.ecs.texture_cache,
            &wgpu_app.dynamic_uniform_buffer,
            &wgpu_app.dynamic_bind_group,
            wgpu_app.dynamic_alignment,
            &lights,
            &wgpu_app.light_buffer,
        );
    }
}

impl ApplicationHandler for App {
    // Начало кадра: накапливаем ввод за текущий кадр
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.input.step();
    }

    // Окно готово к работе: создаём окно, GPU-контекст и поверхность
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attr = Window::default_attributes()
            .with_title("CYS — Create your Shop")
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = event_loop.create_window(window_attr).unwrap();
        // Окно живёт всё время работы приложения, поэтому его можно утечь в 'static
        let window: &'static Window = Box::leak(Box::new(window));

        // GPU-контекст (Instance+Adapter+Device+Queue+пайплайны) создаём ОДИН раз и
        // переиспользуем при каждом последующем resume. На Android `resumed` вызывается
        // повторно при смене поверхности/жизненном цикле; пересоздание всего wgpu-контекста
        // каждый раз уничтожает и заново создаёт VkInstance/VkDevice — из-за этого эмулятор
        // сыплет "Failed to find ColorBuffer", а на устройствах это лишняя задержка и риск
        // потери контекста.
        if self.wgpu_app.is_none() {
            self.wgpu_app = Some(WgpuApp::new(window));
        }
        let wgpu_app = self.wgpu_app.as_mut().unwrap();

        // Поверхность привязана к нативному окну и пересоздаётся при каждом resume.
        let surface = wgpu_app.instance.create_surface(window).expect("Failed to create surface");
        let size = window.inner_size();
        wgpu_app.config.width = size.width;
        wgpu_app.config.height = size.height;
        surface.configure(&wgpu_app.device, &wgpu_app.config);
        wgpu_app.depth_buffer.resize(&wgpu_app.device, size);

        self.window = Some(window);
        self.surface = Some(surface);
    }

    // Переход в фон / уничтожение нативного окна: сбрасываем только поверхность.
    // Instance/Device/пайплайны оставляем живыми, чтобы не пересоздавать GPU-контекст.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface = None;
    }

    // Оконные события: передаём их в накопитель ввода
    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        self.input.process_window_event(&event);

        match &event {
            // Запрос на отрисовку кадра — запускаем полный цикл render
            WindowEvent::RedrawRequested => {
                self.render();
            }
            // Окно изменило размер: пересоздаём конфиг и буфер глубины
            WindowEvent::Resized(new_size) if new_size.width > 0 && new_size.height > 0 => {
                if let Some(ref mut wgpu_app) = self.wgpu_app {
                    if let Some(ref surface) = self.surface {
                        wgpu_app.config.width = new_size.width;
                        wgpu_app.config.height = new_size.height;
                        surface.configure(&wgpu_app.device, &wgpu_app.config);
                        wgpu_app.depth_buffer.resize(&wgpu_app.device, *new_size);
                    }
                }
            }
            _ => {}
        }
    }

    // События клавиатуры на уровне устройства (например, переключение раскладки)
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        self.input.process_device_event(&event);
    }

    // Между кадрами: завершаем обработку ввода и решаем, продолжать ли работу
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.input.end_step();

        // Выход по запросу закрытия окна или по действию сцены
        if self.input.close_requested() || self.quit_requested {
            event_loop.exit();
            return;
        }
        if let (Some(ref window), Some(_)) = (self.window.as_ref(), self.surface.as_ref()) {
            // Запрашиваем следующий кадр (непрерывный рендер), только когда
            // поверхность готова (в фоне surface=None — рендерить нечего).
            window.request_redraw();
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn run() {
    // Путь к шрифту берём из scripts/config.lua (поле font_path).
    // Создаём цикл событий winit
    let event_loop = EventLoop::new().unwrap();

    // Собираем приложение (окно и GPU появятся в resumed)
    let mut app = App::new(Box::new(DesktopInput::new()));

    // Запуск главного цикла приложения (блокирует до выхода)
    let _ = event_loop.run_app(&mut app);
}

// Android-точка входа (компилируется только под android-таргет)
#[cfg(target_os = "android")]
mod android;