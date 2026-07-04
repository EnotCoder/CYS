use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;
mod api_components;
mod input;
mod ecs;
mod scene;
mod map;
mod inventory;
mod data;
mod npc;
mod constants;
mod util;
mod ui;

pub use map::load_map_to_ecs;
use crate::constants::*;

use api_components::*;
use ecs::*;
use ui::fps::FpsCounter;

struct App {
    window: Option<&'static Window>,
    wgpu_app: Option<WgpuApp>,
    surface: Option<wgpu::Surface<'static>>,
    text_renderer: ui::text_renderer::TextRenderer,
    scene_manager: scene::SceneManager,
    input: winit_input_helper::WinitInputHelper,
    fps_counter: FpsCounter,
    quit_requested: bool,
}

impl App {
    fn render(&mut self) {
        let Some(surface) = &self.surface else { return };
        let Some(ref window) = self.window else { return };

        let window_size = (
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );

        // Шаг 1: update сцены — временно берём wgpu_app immutably
        let action = {
            let Some(ref wgpu_app) = self.wgpu_app else { return };
            let scene = self.scene_manager.scenes.get_mut(&self.scene_manager.current).unwrap();
            scene.update(
                &mut self.scene_manager.ecs,
                &self.input,
                window_size,
                &mut self.text_renderer,
                &wgpu_app.device,
                &wgpu_app.queue,
            )
        };

        // Шаг 2: обрабатываем action — borrow wgpu_app освобождён
        match action {
            scene::SceneAction::Switch(name) => {
                self.scene_manager.switch_to(&name, &mut self.text_renderer);
            }
            scene::SceneAction::Quit => {
                self.quit_requested = true;
            }
            scene::SceneAction::VsyncToggle(enabled) => {
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
            scene::SceneAction::None => {}
        }

        // Шаг 3: рендер — снова берём wgpu_app immutably
        let Some(ref wgpu_app) = self.wgpu_app else { return };

        let fps = self.fps_counter.tick();
        self.scene_manager.update_fps(fps, &mut self.text_renderer, &wgpu_app.device, &wgpu_app.queue);

        let ms = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().map_size();
        let (cam_x, cam_y) = self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().camera_offset();
        let aspect = window_size.0 / window_size.1;
        let size_data = Size { map_size: ms, aspect, offset_x: cam_x, offset_y: cam_y };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));

        let ui_uniforms = UiUniforms { size: 1.0, aspect, _padding: [0.0; 2] };
        wgpu_app
            .queue
            .write_buffer(&wgpu_app.ui_uniform_buffer, 0, bytemuck::cast_slice(&[ui_uniforms]));

        let vis_w = 2.0 * aspect / (SHADER_SCALE * ms);
        let vis_h = 2.0 / (SHADER_SCALE * ms);
        let bounds = Some((cam_x - vis_w/2.0, cam_x + vis_w/2.0, cam_y - vis_h/2.0, cam_y + vis_h/2.0));
        let (map_sprites, carpet_sprites, decor_sprites, npc_sprites, cursor_sprites, ui_sprites) =
            self.scene_manager.scenes.get(&self.scene_manager.current).unwrap().sprites(&self.scene_manager.ecs, bounds);

        render(
            surface,
            &wgpu_app.device,
            &wgpu_app.queue,
            &wgpu_app.render_pipeline,
            &wgpu_app.transparent_pipeline,
            &wgpu_app.depth_buffer.view,
            &map_sprites,
            &carpet_sprites,
            &decor_sprites,
            &npc_sprites,
            &cursor_sprites,
            &ui_sprites,
            &wgpu_app.size_bind_group,
            &wgpu_app.ui_bind_group,
            &mut self.scene_manager.ecs.sprite_cache,
            &wgpu_app.dynamic_uniform_buffer,
            &wgpu_app.dynamic_bind_group,
            wgpu_app.dynamic_alignment,
        );
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.input.step();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attr = Window::default_attributes()
            .with_title("CYS — Create your Shop")
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = event_loop.create_window(window_attr).unwrap();
        let window: &'static Window = Box::leak(Box::new(window));

        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        let surface = wgpu_app.instance.create_surface(window).expect("Failed to create surface");
        let config = surface_config(wgpu_app.surface_format, window.inner_size().width, window.inner_size().height);
        surface.configure(&wgpu_app.device, &config);

        self.window = Some(window);
        self.wgpu_app = Some(wgpu_app);
        self.surface = Some(surface);
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        self.input.process_window_event(&event);

        match &event {
            WindowEvent::RedrawRequested => {
                self.render();
            }
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

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        self.input.process_device_event(&event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.input.end_step();

        if self.input.close_requested() || self.quit_requested {
            event_loop.exit();
            return;
        }
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut app = App {
        window: None,
        wgpu_app: None,
        surface: None,
        text_renderer: ui::text_renderer::TextRenderer::new("font.otf"),
        scene_manager: scene::SceneManager::new(),
        input: winit_input_helper::WinitInputHelper::new(),
        fps_counter: FpsCounter::new(),
        quit_requested: false,
    };

    let _ = event_loop.run_app(&mut app);
}