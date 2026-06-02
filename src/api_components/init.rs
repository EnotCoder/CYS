use winit::window::Window;
use wgpu::*;
use wgpu::util::DeviceExt;
use crate::init_buffers;
use crate::Vertex;
use crate::DepthBuffer;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Size {
    pub map_size: f32,
}

#[allow(dead_code)]
pub struct WgpuApp {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,

    pub uniform_buffer: wgpu::Buffer,
    pub depth_stencil: wgpu::DepthStencilState,
    pub size_buffer: wgpu::Buffer,
    pub transparent_depth_stencil: wgpu::DepthStencilState,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_buffer: DepthBuffer,

    pub render_pipeline: wgpu::RenderPipeline,
    pub transparent_pipeline: wgpu::RenderPipeline,
    pub config: wgpu::SurfaceConfiguration,

    pub size_bind_group: wgpu::BindGroup,
}

impl WgpuApp{
    pub async fn new(
        window: &Window,
    ) -> Self{
        //instance (экземпляр)
        //with defalt settings
        let instance = wgpu::Instance::new(InstanceDescriptor::default());

        //поверхность
        let surface = instance.create_surface(window)
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
    
        let buffers = init_buffers(
            window_size,
            translation,
            rotation,
            &device,
        );

        let uniform_buffer = buffers.uniform_buffer;
        let depth_stencil = buffers.depth_stencil;
        let depth_buffer = buffers.depth_buffer;
        let transparent_depth_stencil = buffers.transparent_depth_stencil;
        let bind_group_layout = buffers.bind_group_layout;
        let bind_group = buffers.bind_groupprojection;
        let texture_bind_group_layout = buffers.texture_bind_group_layout;

        //shaders
        //получаем код шейдера
        let shader_code = include_str!("shaders.wgsl");
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


        let map_size = 1.0;

        let size = Size{map_size};

        // Создаём uniform buffer
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });


        // Создаём bind group layout (описывает доступ к uniform буферу в шейдере)
        let bind_group_layout_0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let size_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout_0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: size_buffer.as_entire_binding(),
                },
            ],
        });

        //Render pipeline
        //PipelineLayout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout, &bind_group_layout_0],
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
            depth_stencil: Some(depth_stencil.clone()),
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
            depth_stencil: Some(transparent_depth_stencil.clone()),  // ← используем transparent
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

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
        
        Self{
            instance,
            device,
            queue,
            surface_format,

            uniform_buffer,
            size_buffer,
            depth_stencil,
            transparent_depth_stencil,
            bind_group_layout,
            bind_group,
            texture_bind_group_layout,
            depth_buffer,

            render_pipeline,
            transparent_pipeline,
            config,
            size_bind_group,
        }
    }
}
