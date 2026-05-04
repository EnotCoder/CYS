use winit::{
    event::{Event,WindowEvent},
    event_loop::{ControlFlow,EventLoop},
    window::WindowBuilder,
};
use wgpu::{util::DeviceExt, *};
use tokio;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}


#[tokio::main]
async fn main() {
    
    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("game")
        .build(&event_loop)
        .unwrap();
    
    init(

    );

    //main loop
    let window_id = window.id();
    // main loop
    event_loop.run(|event, event_loop_target| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window_id => {
                event_loop_target.exit();
            }

            Event::AboutToWait => {
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                render(
                    &surface,&device,&queue,&render_pipeline,
                    &vertex_buffer,&mut vertices
                );
            }

            // Обработка изменения размера окна
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                window_id,
            } if window_id == window.id() => {
                // Обновляем конфигурацию surface под новый размер
                let config = SurfaceConfiguration {
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    format: surface_format,
                    width: new_size.width,
                    height: new_size.height,
                    present_mode: PresentMode::Fifo,
                    alpha_mode: CompositeAlphaMode::Auto,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                };
                surface.configure(&device, &config);
            }

            // Игнорируем все остальные события
            _ => (),
        }
    });

}

fn render(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertices: &mut [Vertex; 3],
){
    vertices[0].position[0] += 0.001;
    // ПЕРЕсоздаём буфер вершин
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

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
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Устанавливаем конвейер
        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        // Отрисовываем 3 вершины (треугольник), 1 экземпляр
        render_pass.draw(0..3, 0..1);
    }

    // Завершаем запись команд
    let command_buffer = encoder.finish();
    // Отправляем команды на выполнение (как итератор)
    queue.submit(std::iter::once(command_buffer));
    // Показываем кадр
    frame.present();
}