// egui_manager.rs
use egui::{Context, Visuals, epaint::Shadow};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use winit::window::Window;

pub struct EguiManager {
    pub context: Context,      // Контекст egui - "мозг" UI
    state: State,              // Состояние для связи с winit
    renderer: Renderer,        // Рендерер для отрисовки UI
}

impl EguiManager {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> Self {
        let egui_context = Context::default();
        let id = egui_context.viewport_id();
        
        // Настройка визуального стиля
        let visuals = Visuals {
            window_rounding: egui::Rounding::same(5.0),
            window_shadow: Shadow::NONE, //shadow
            ..Default::default()
        };
        egui_context.set_visuals(visuals);
        
        //Winit connect
        
        let state = State::new(egui_context.clone(), id, window, None, None);
        //Create renderer
        let renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );
        
        Self {
            context: egui_context,
            state,
            renderer,
        }
    }
    //принимает событие от winit и обрабатывает
    pub fn handle_input(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }
    
    pub fn draw<F>(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        run_ui: F,
    ) where
        F: FnOnce(&Context),
    {   
        //Получение ввода
        let raw_input = self.state.take_egui_input(window);
        //Обновление контекста и UI
        let full_output = self.context.run(raw_input, |_ui| {
            run_ui(&self.context);
        });
        
        //Обработка платформенного вывода
        self.state
            .handle_platform_output(window, full_output.platform_output);
        
        //Тесселяция (преобразование в треугольники)
        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        
        //Обновление текстур
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        //Обновление буферов
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        
        //Создание render pass для UI
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            label: Some("egui render pass"),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        //Рендер UI
        self.renderer.render(&mut render_pass, &tris, &screen_descriptor);
        drop(render_pass);
        //Очистка текстур
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}