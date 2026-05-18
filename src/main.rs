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
mod render;
mod models;
mod texture;
mod egui_manager;
mod ui_panels;

use egui_manager::EguiManager;
use ui_panels::UiState;
use egui_wgpu::ScreenDescriptor;


use texture::*;
use buffers::*;
use render::*;
use models::*;


use std::env;

#[tokio::main]
async fn main() {
    //get model path
    let model_path = {
        let args:Vec<String> = env::args().collect();
        if args.len() > 1{args[1].clone()}
        else {"null.obj".to_string()}
    };
    //get texture path
    let texture_path = {
        let args:Vec<String> = env::args().collect();
        if args.len() > 2{args[2].clone()}
        else {"null.png".to_string()}
    };



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

    //запрашиваем физ устройство
    let addapter_future = instance.request_adapter(&addapter_option);
    //ожидаем
    let addapter = pollster::block_on(addapter_future).unwrap();

    println!("{}",addapter.get_info().name);
    
    //Log device
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

    let caps = surface.get_capabilities(&addapter);
    let surface_format = caps.formats[0];

    //init buffers

    let mut translation = [0.0, 0.0, 4.5, 0.0];
    let rotation = [-0.2, 0.0, 0.0, 0.0];
    let window_size = window.inner_size();

    let mut buffers = init_buffers(
        window_size,
        translation,
        rotation,
        &device,
    );

    let mut projection = buffers.projection;
    let uniform_buffer = &buffers.uniform_buffer;
    let depth_stencil = buffers.depth_stencil;
    let bind_group_layout = &buffers.bind_group_layout;
    let bind_group = &buffers.bind_groupprojection;

    //Load main scene

    let load_model = ModelInstance::new(&model_path, &device, &queue,
        translation, [0.0, 0.0, 0.0, 0.0],
        rotation, projection, &texture_path);
    
    let fon_model = ModelInstance::new("models/fon.obj", &device, &queue,
        translation, [0.0, 0.0, 15.0, 0.0],
        [0.0, 0.0, 0.0, 0.0], projection, "tex/fon_texture.png");

    let mut models = vec![load_model, fon_model];


    //shaders
    //получаем код шейдера
    let shader_code = include_str!(".././src/shaders.wgsl");
    //shader object
    //описание шейдера
    let description = wgpu::ShaderModuleDescriptor {
        //отладка
        label : None,
        //.into() - преобразует &str в Cow<'_, str> <- (владеть или читать)
        source : wgpu::ShaderSource::Wgsl(shader_code.into()),
    };
    //Компилирует шейдер для GPU
    let shader_module = device.create_shader_module(description);


    //texture_bind_gruuo_layout
    let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });


    //Render pipeline
    //PipelineLayout
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
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
                        format: wgpu::VertexFormat::Float32x2,
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
    let speed = 0.1;
    let rotation_speed = 0.01;

    //INIT UV
    let mut egui_manager = EguiManager::new(
        &device,
        surface_format,
        None,  // depth format
        1,     // samples
        &window,
    );

    let mut ui_state = UiState::new(model_path, texture_path);

    // main loop
    let _ = event_loop.run(|event, event_loop_target| {

        if let Event::WindowEvent { event, window_id } = &event {
            if *window_id == window.id() {
                egui_manager.handle_input(&window, event);
            }
        }

        //Input
        if input.update(&event) {
            let scroll_delta = input.scroll_diff();

            if scroll_delta.1 > 0.0 && models[0].translation_base[2] < 5.0{
                models[0].translation_base[2] += 0.5;
            }
            if scroll_delta.1 < 0.0 && models[0].translation_base[2] > -2.0{
                models[0].translation_base[2] -= 0.5;
            }

            if input.key_pressed(KeyCode::F1) {
                ui_state.toggle_panel();
            }
        }
            
        for model in &mut models{
            let new_pos = [
                model.translation_base[0] + translation[0],
                model.translation_base[1] + translation[1],
                model.translation_base[2] + translation[2],
                model.translation_base[3] + translation[3],
            ];

            model.translation = new_pos;
            model.update_transform(&queue, projection, 1);
        }

        models[0].update_transform(&queue, projection, ui_state.use_texture as i32);

        let uniforms = Uniforms { translation, rotation, projection, use_texture: 1 ,_padding: [0.0; 3],};
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        //Render

        match event {
            //Exit
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window_id => {
                event_loop_target.exit();
            }
            //Redraw window
            Event::AboutToWait => {
                window.request_redraw();
            }
            //Render
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                render(
                    &surface,&device,&queue,&render_pipeline,
                    &models,bind_group,&buffers.depth_buffer.view,
                    &mut egui_manager,
                    &window,
                    |ctx| ui_state.render(ctx),
                );
                models[0].rotation[1] += ui_state.rotation_speed;
            }

            //Window resize
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

                buffers.depth_buffer.resize(&device, new_size);

                let new_aspect = new_size.width as f32 / new_size.height as f32;
                    projection = create_perspective_matrix(new_aspect, 
                    std::f32::consts::PI / 4.0, 0.1, 100.0);
            }

            // Игнорируем все остальные события
            _ => (),
        }
    });

}
