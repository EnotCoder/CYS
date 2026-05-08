use winit::{
    event::{Event,WindowEvent},
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use wgpu::{util::DeviceExt, *};
use winit::dpi::PhysicalSize;
use tokio;

use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

mod buffers;
use buffers::*;

mod render;
use render::*;

//triangle info
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
    let surface = instance.create_surface(&window)
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

    //init buffers
    let mut translation = [0.0, 0.0, 5.0, 0.0];
    let window_size = window.inner_size();
    let speed = 0.01;

    //VBO   
    let vertices = [
        make_triangle(-0.5, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.0, 0.6),
        make_triangle( 0.5, -0.5, 0.0, 0.5, 0.5, 0.0, -0.5, 0.5, 0.0, 0.0, 0.0, 1.0),
    ].concat();

    //EBO
    let mut indices: [u16; 6] = [
        0, 1, 2,
        3, 4, 5,
    ];

    let buffers = init_buffers(
        window_size,
        translation,
        &device,
        &vertices,
        &indices,
    );

    let mut projection = buffers.projection;
    let uniform_buffer = buffers.uniform_buffer;
    let mut depth_buffer = buffers.depth_buffer;
    let depth_stencil = buffers.depth_stencil;
    let bind_group_layout = buffers.bind_group_layout;
    let bind_groupprojection = buffers.bind_groupprojection;

    let vertex_buffer = buffers.vertex_buffer;
    let index_buffer = buffers.index_buffer;

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

    //surface_format
    let surface_format = TextureFormat::Bgra8UnormSrgb;

    //color_target 
    // dont use
    let _color_target = wgpu::ColorTargetState {
        format: surface_format,
        blend: Some(BlendState::REPLACE),
        write_mask: ColorWrites::ALL,
    };


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

    //main loop vars
    let mut input = WinitInputHelper::new();

    // main loop
    let _ = event_loop.run(|event, event_loop_target| {

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
                    &bind_groupprojection,
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