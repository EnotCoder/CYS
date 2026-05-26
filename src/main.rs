use winit::{
    event::{Event,WindowEvent},
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use wgpu::*;
use winit::dpi::PhysicalSize;
use tokio;

use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

mod buffers;
mod render;
mod texture;
mod egui_manager;
mod ui_panels;
mod sprite_manager;

use egui_manager::EguiManager;
use ui_panels::UiState;


use buffers::*;
use render::*;
use sprite_manager::*;

struct GameObjects {
    cursor: Sprite,
    map: Vec<Sprite>,
    decor: Vec<Sprite>,
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

    //init buffers

    let translation = [0.0, 0.0, 0.0, 0.0];
    let rotation = [0.0, 0.0, 0.0, 0.0];
    let window_size = window.inner_size();
 
    let mut buffers = init_buffers(
        window_size,
        translation,
        rotation,
        &device,
    );

    let uniform_buffer = &buffers.uniform_buffer;
    let depth_stencil = buffers.depth_stencil;
    let transparent_depth_stencil = buffers.transparent_depth_stencil;
    let bind_group_layout = &buffers.bind_group_layout;
    let bind_group = &buffers.bind_groupprojection;
    let texture_bind_group_layout = &buffers.texture_bind_group_layout;

    // Создаём блок

    let mut cursor: Sprite = Sprite::new(&device, &queue, "./tex/def_cursor.png", [0,0], 1);
    cursor.translation = [4.0,4.0,0.0,1.0];
    cursor.build_buffers(&device);

    let mut map:Vec<Sprite> = Vec::new();
    let decor:Vec<Sprite> = Vec::new();

    for i in 0..10{
        for j in 0..10{
            let mut block: Sprite = Sprite::new(&device, &queue, "tex/floor.png", [0,0], 2);
            block.translation = [i as f32 - 4.0, j as f32 - 4.0, 0.0, 1.0];
            block.build_buffers(&device);

            map.push(block);
        }
    }

    let mut game : GameObjects = GameObjects {cursor, map, decor};

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


    //Render pipeline
    //PipelineLayout
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[], // константы, которые можно быстро обновлять
    });
    let caps = surface.get_capabilities(&addapter);
    let surface_format = caps.formats[0];


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
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,   // источник = альфа
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, // фон = 1-альфа
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
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

    let transparent_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Transparent Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    }
                ]
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),  // прозрачные - со смешиванием
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(transparent_depth_stencil),  // ← используем transparent
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut config = SurfaceConfiguration {
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

    //0 - Standart
    //1 - build
    //2 - Break

    let mut mode = 0;

    //INIT UV
    let mut egui_manager = EguiManager::new(
        &device,
        surface_format,
        None,  // depth format
        1,     // samples
        &window,
    );

    let mut ui_state = UiState::new();

    // main loop
    let _ = event_loop.run(|event, event_loop_target| {

        if let Event::WindowEvent { event, window_id } = &event {
            if *window_id == window.id() {
                egui_manager.handle_input(&window, event);
            }
        }

        //Input
        if input.update(&event) {
            if input.key_pressed(KeyCode::KeyF) {
                match mode{
                    0 => {},
                    1 =>{
                        let mut block: Sprite = Sprite::new(&device, &queue, "tex/decor.png", [0,0], 2);
                        block.translation = game.cursor.translation;
                        block.build_buffers(&device);

                        game.decor.push(block);
                    },
                    2 => {
                        for i in 0..game.map.len(){
                            if game.decor[i].translation == game.cursor.translation{
                                game.decor.remove(i);
                                break;
                            }
                        }
                    },

                    _ => {} 
                }
            }

            if input.key_pressed(KeyCode::Tab) {
                if mode == 2{
                    mode = 0
                }else{
                    mode += 1;
                }

                match mode{
                    0 => game.cursor.update_texture(&device, &queue, "./tex/def_cursor.png"),
                    1 => game.cursor.update_texture(&device, &queue, "./tex/cursor.png"),
                    2 => game.cursor.update_texture(&device, &queue, "./tex/del_cursor.png"),
                    _ => ()
                }
            }

            if input.key_pressed(KeyCode::KeyW) {
                if game.cursor.translation[1] < 4.0{
                    game.cursor.translation[1] += 1.0;
                    game.cursor.build_buffers(&device);
                }
            }

            if input.key_pressed(KeyCode::KeyS) {
                if game.cursor.translation[1] > -4.0{
                    game.cursor.translation[1] -= 1.0;
                    game.cursor.build_buffers(&device);
                }
            }

            if input.key_pressed(KeyCode::KeyA) {
                if game.cursor.translation[0] > -4.0{
                    game.cursor.translation[0] -= 1.0;
                    game.cursor.build_buffers(&device);
                }
            }

            if input.key_pressed(KeyCode::KeyD) {
                if game.cursor.translation[0] < 4.0{
                    game.cursor.translation[0] += 1.0;
                    game.cursor.build_buffers(&device);
                }
            }
        }

        let uniforms = Uniforms { translation, rotation, _padding: [0.0; 3],};
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

                let mut opaque_models = vec![];
                opaque_models.extend(game.map.iter());

                let mut transparent_models = vec![];
                transparent_models.extend(game.decor.iter());
                transparent_models.push(&game.cursor);

                render(
                    &surface, &device, &queue, &render_pipeline, &transparent_pipeline,
                    bind_group, &buffers.depth_buffer.view,
                    &mut egui_manager,
                    &window,
                    |ctx| ui_state.render(ctx),
                    &opaque_models, &transparent_models
                );
            }

            //Window resize
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                window_id,
            } if window_id == window.id() => {
                // Обновляем существующую конфигурацию
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(&device, &config);
                
                buffers.depth_buffer.resize(&device, new_size);

                // Запрашиваем перерисовку
                window.request_redraw();
            }

            // Игнорируем все остальные события
            _ => (),
        }
    });

}
