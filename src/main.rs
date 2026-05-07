use winit::{
    event::{Event,WindowEvent},
    event_loop::{ControlFlow,EventLoop},
    window::WindowBuilder,
};
use wgpu::{util::DeviceExt, *};
use winit::dpi::PhysicalSize;
use tokio;

use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

fn make_triangle(
    x1: f32, y1: f32, z1: f32,
    x2: f32, y2: f32, z2: f32,
    x3: f32, y3: f32, z3: f32,
    r: f32, g: f32, b: f32,
) -> [Vertex; 3] {
    [
        Vertex { position: [x1, y1, z1], color: [r, g, b] },
        Vertex { position: [x2, y2, z2], color: [r, g, b] },
        Vertex { position: [x3, y3, z3], color: [r, g, b] },
    ]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    translation: [f32; 4],
    projection: [f32; 16],
}

struct DepthBuffer {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthBuffer {
    fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        Self { texture, view }
    }
    
    fn resize(&mut self, device: &wgpu::Device, new_size: winit::dpi::PhysicalSize<u32>) {
        *self = Self::new(device, new_size);
    }
}

// Создаём матрицу перспективы
fn create_perspective_matrix(aspect: f32, fov: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov * 0.5).tan();
    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, far / (far - near), 1.0,
        0.0, 0.0, -far * near / (far - near), 0.0,
    ]
}

#[tokio::main]
async fn main() {
    //основной цикл winit
    let event_loop = EventLoop::new().unwrap();


    //winit window
    let window = WindowBuilder::new()
        .with_title("game")
        .with_inner_size(PhysicalSize::new(800, 800))
        .build(&event_loop)
        .unwrap();
    
    //instance (экземпляр)
    //with defalt settings
    let instance = wgpu::Instance::new(InstanceDescriptor::default());

    //поверхность
    //usafe
    let surface = unsafe { instance.create_surface(&window) }
        .expect("Failed to create surface");

    //addapter/physical_device

    //опции выбора видеокарты
    let addapter_option = wgpu::RequestAdapterOptions {
        //выбирает адаптер, совместимый с этой поверхностью (обычно дискретная видеокарта)
        compatible_surface : Some(&surface),
        //всё остальное поумолчанию
        ..Default::default()
    };

    let addapter_future = instance.request_adapter(&addapter_option);

    let addapter = pollster::block_on(addapter_future).unwrap();

    println!("{}",addapter.get_info().name);
    
    //device
    //let device_description = wgpu::DeviceDescriptor::default();
    let (device, queue) = addapter
    .request_device(
        &DeviceDescriptor { //настройки устройства
            //какие плагины необходимы
            required_features: Features::empty(),
            //минимальные требования
            required_limits: Limits::default(),
            //Отладка
            label: None,
        },
        None,
    )
    .await
    .unwrap();


    //shaders
    //получаем код шейдера
    let shader_code = include_str!(".././src/shaders.wgsl");
    //shader object
    //описание шейдера
    let description = wgpu::ShaderModuleDescriptor {
        //отладка
        label : None,
        //.into() - преобразует &str в Cow<'_, str>
        source : wgpu::ShaderSource::Wgsl(shader_code.into()),
    };
    //Компилирует шейдер для GPU
    let shader_module = device.create_shader_module(description);
    

    // Начальная позиция квадрата
    let mut translation = [0.0, 0.0, 5.0, 0.0];
    let window_size = window.inner_size();

    let aspect = window_size.width as f32 / window_size.height as f32;
    let mut projection = create_perspective_matrix(aspect, std::f32::consts::PI / 4.0, 0.1, 100.0);

    let uniforms = Uniforms { 
        translation: translation,
        projection: projection,
    };

    // Создаём uniform buffer
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });


    // Создаём bind group layout (описывает доступ к uniform буферу в шейдере)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX, // Доступен только в вершинном шейдере
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Создаём bind group (связывает uniform буфер с шейдером)
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    // Создаём depth texture
    let mut depth_buffer = DepthBuffer::new(&device, window_size);

    // Настройка depth_stencil для render pipeline
    let depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };

    //surface_format
    let surface_format = TextureFormat::Bgra8UnormSrgb;

    //color_target
    let color_target = wgpu::ColorTargetState {
        format: surface_format,
        blend: Some(BlendState::REPLACE),
        write_mask: ColorWrites::ALL,
    };

    let color_targets = [Some(color_target)];

    //Render pipeline
    //PipelineLayout
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout], // группы привязки (текстуры, буферы)
        push_constant_ranges: &[], // константы, которые можно быстро обновлять
    });


    let description = wgpu::RenderPipelineDescriptor {
        label : Some("Render Pipeline"),
        vertex : wgpu::VertexState {
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,// смещение на 0 байт
                        shader_location: 0,  // @location(0) в шейдере
                        format: wgpu::VertexFormat::Float32x3,  // position: [f32; 2]
                    },
                    wgpu::VertexAttribute {
                        offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,// смещение 8 байт
                        shader_location: 1,  // @location(1) в шейдере
                        format: wgpu::VertexFormat::Float32x3,  // color: [f32; 3]
                    }
                ]
            }],
            module : &shader_module,
            entry_point : "vs_main",
        },

        fragment : Some(wgpu::FragmentState {
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            module : &shader_module,
            entry_point : "fs_main",
        }),
        
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList, // список треугольников
            strip_index_format: None, // не используем полоски
            front_face: FrontFace::Ccw, // против часовой стрелки = лицевая
            cull_mode: None, // не отсекаем грани
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill, // заливаем цветом (не каркасный режим)
            conservative: false, // не использовать консервативный растеризатор
        },

        layout : Some(&pipeline_layout),
        depth_stencil: Some(depth_stencil),
        multisample : Default::default(),
        multiview : None,
    };

    let render_pipeline = device.create_render_pipeline(&description);

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT, // использовать как цель рендеринга
        format: surface_format, // формат пикселей (BGRA8)
        width: window_size.width, // ширина окна
        height: window_size.height, // высота окна
        present_mode: PresentMode::Fifo, //вертикальная синхронизация
        alpha_mode: CompositeAlphaMode::Auto, // альфа-канал автоматический
        view_formats: vec![], // дополнительные форматы для текстур
        desired_maximum_frame_latency: 2, // задержка кадров (2 = баланс)
    };

    surface.configure(&device, &config);


    //Vbo
       
    let mut vertices = [
        make_triangle(-0.5, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.0, 0.6),
        make_triangle( 0.5, -0.5, 0.0, 0.5, 0.5, 0.0, -0.5, 0.5, 0.0, 0.0, 0.0, 1.0),
    ].concat();

    // Создаём буфер вершин
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    //EBO
    // Индексы для двух треугольников (6 индексов)
    let mut indices: [u16; 6] = [
        0, 1, 2,  // первый треугольник
        3, 4, 5,  // второй треугольник
    ];

    // Создаём буфер индексов
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    
    

    //main loop vars
    let window_id = window.id();
    let mut input = WinitInputHelper::new();

    // Добавьте скорость и позицию
    let mut speed = 0.01;
    // main loop
    event_loop.run(|event, event_loop_target| {

        // Передаём каждое событие в input helper
        if input.update(&event) {

            // В event_loop.run, внутри if input.update(&event):
            if input.key_held(KeyCode::KeyA) || input.key_held(KeyCode::ArrowLeft) {
                translation[0] -= speed;
            }
            if input.key_held(KeyCode::KeyD) || input.key_held(KeyCode::ArrowRight) {
                translation[0] += speed;
            }
            if input.key_held(KeyCode::KeyW) || input.key_held(KeyCode::ArrowUp) {
                translation[2] += speed;
            }
            if input.key_held(KeyCode::KeyS) || input.key_held(KeyCode::ArrowDown) {
                translation[2] -= speed;
            }
        }

        // Обновляем uniform buffer
        let uniforms = Uniforms { translation, projection,};
        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

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
                    &vertex_buffer,&index_buffer,
                    &mut indices,
                    &bind_group,
                    &depth_buffer.view,
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

                depth_buffer.resize(&device, new_size);

                let new_aspect = new_size.width as f32 / new_size.height as f32;
                    projection = create_perspective_matrix(new_aspect, 
                    std::f32::consts::PI / 4.0, 0.1, 100.0);
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
    index_buffer: &wgpu::Buffer,
    indices : &mut [u16;6],
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