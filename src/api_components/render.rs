
use crate::egui_manager::EguiManager;
use egui_wgpu::ScreenDescriptor;
use crate::Sprite;

// render.rs - добавьте эту функцию
pub fn render(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_pipeline: &wgpu::RenderPipeline,
    transparent_pipeline: &wgpu::RenderPipeline,
    depth_view: &wgpu::TextureView,
    egui_manager: &mut EguiManager,
    window: &winit::window::Window,
    run_ui: impl FnOnce(&egui::Context),

    opaque_models: &[&Sprite],      // непрозрачные
    transparent_models: &[&Sprite], // прозрачные
    ui_model: &[&Sprite],

    bind_group: &wgpu::BindGroup,
    size_bind_group: &wgpu::BindGroup,
    ui_bind_group: &wgpu::BindGroup, 
) {
    let frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(_) => return,
    };
    
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });
    
    // 1 render
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.2,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        
        for model in opaque_models{
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, &model.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &model.texture_bind_group, &[]);
            render_pass.set_bind_group(2, &size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(model.index_buffer.slice(..), model.index_format );
            render_pass.draw_indexed(0..model.index_count, 0, 0..1);
        }
    }
    // 2 render
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Transparent Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,  // ← Load, не Clear! Сохраняем то, что уже нарисовано
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,  // ← Load - сохраняем depth buffer
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(transparent_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        
        // Для прозрачных объектов - рисуем от дальних к ближним (обратная сортировка по Z)
        let mut sorted_transparent = transparent_models.to_vec();
        sorted_transparent.sort_by(|a, b| 
            b.translation[2].partial_cmp(&a.translation[2]).unwrap()
        );
        
        for model in &sorted_transparent {
            render_pass.set_bind_group(0, &model.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &model.texture_bind_group, &[]);
            render_pass.set_bind_group(2, &size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(model.index_buffer.slice(..), model.index_format );
            render_pass.draw_indexed(0..model.index_count, 0, 0..1);
        }

        let ui_transparent = ui_model.to_vec();
        sorted_transparent.sort_by(|a, b| 
            b.translation[2].partial_cmp(&a.translation[2]).unwrap()
        );

        for model in &ui_transparent {
            render_pass.set_bind_group(0, &model.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &model.texture_bind_group, &[]);
            render_pass.set_bind_group(2, &ui_bind_group, &[]);
            render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(model.index_buffer.slice(..), model.index_format );
            render_pass.draw_indexed(0..model.index_count, 0, 0..1);
        }
    }
    
    // UI
    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [frame.texture.width(), frame.texture.height()],
        pixels_per_point: window.scale_factor() as f32,
    };
    
    egui_manager.draw(device, queue, &mut encoder, window, &view, screen_descriptor, run_ui);
    
    queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}