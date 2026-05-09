use wgpu::{util::DeviceExt, *};

pub fn render(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    indices : &Vec<u16>,
    bind_group: &wgpu::BindGroup,
    depth_view: &wgpu::TextureView,
){

    // Получаем текущий кадр (с обработкой ошибок)
    let frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("Failed to get current texture: {:?}", e);
            return;
        }
    };
    let view = frame.texture.create_view(&TextureViewDescriptor::default());

    // Создаём командный энкодер
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    // Начинаем рендер‑пасс (с полным описанием)
    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::WHITE),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Устанавливаем конвейер
        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, bind_group, &[]);
        // Отрисовываем 3 вершины (треугольник), 1 экземпляр
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    // Завершаем запись команд
    let command_buffer = encoder.finish();
    // Отправляем команды на выполнение (как итератор)
    queue.submit(std::iter::once(command_buffer));
    // Показываем кадр
    frame.present();
}