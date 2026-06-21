use winit::window::Window;
use wgpu::*;
use wgpu::util::DeviceExt;
use crate::Vertex;
use crate::DepthBuffer;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Size {
    pub map_size: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiUniforms {
    pub size: f32,
    pub _padding: [f32; 3],
}

#[allow(dead_code)]
pub struct WgpuApp {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,

    pub uniform_buffer: wgpu::Buffer,
    pub depth_stencil: wgpu::DepthStencilState,
    pub transparent_depth_stencil: wgpu::DepthStencilState,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_buffer: DepthBuffer,

    pub render_pipeline: wgpu::RenderPipeline,
    pub transparent_pipeline: wgpu::RenderPipeline,
    pub config: wgpu::SurfaceConfiguration,

    pub size_buffer: wgpu::Buffer,
    pub size_bind_group: wgpu::BindGroup,

    pub ui_uniform_buffer: wgpu::Buffer,
    pub ui_bind_group: wgpu::BindGroup,
}

impl WgpuApp {
    pub async fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let surface = instance.create_surface(window)
            .expect("Failed to create surface");

        let adapter = Self::request_adapter(&instance, &surface).await;
        let (device, queue) = Self::request_device(&adapter).await;
        let surface_format = Self::pick_format(&surface, &adapter);

        let window_size = window.inner_size();
        let translation = [0.0, 0.0, 0.0, 0.0];
        let rotation = [0.0, 0.0, 0.0, 0.0];

        let buffers = crate::init_buffers(window_size, translation, rotation, &device);
        let shader_module = Self::load_shader(&device);

        let size_buffer = Self::create_size_buffer(&device);
        let size_bind_group_layout = Self::create_single_bind_group_layout(&device, "Size Bind Group Layout");
        let size_bind_group = Self::create_bind_group(&device, &size_bind_group_layout, &size_buffer, "Size Bind Group");

        let ui_uniform_buffer = Self::create_ui_buffer(&device);
        let ui_bind_group_layout = Self::create_single_bind_group_layout(&device, "UI Bind Group Layout");
        let ui_bind_group = Self::create_bind_group(&device, &ui_bind_group_layout, &ui_uniform_buffer, "UI Bind Group");

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                &buffers.bind_group_layout,
                &buffers.texture_bind_group_layout,
                &size_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = Self::create_render_pipeline(
            &device, &pipeline_layout, &shader_module,
            surface_format, &buffers.depth_stencil,
        );
        let transparent_pipeline = Self::create_transparent_pipeline(
            &device, &pipeline_layout, &shader_module,
            surface_format, &buffers.transparent_depth_stencil,
        );

        let config = surface_config(surface_format, window_size.width, window_size.height);
        surface.configure(&device, &config);

        Self {
            instance,
            device,
            queue,
            surface_format,
            uniform_buffer: buffers.uniform_buffer,
            depth_stencil: buffers.depth_stencil,
            transparent_depth_stencil: buffers.transparent_depth_stencil,
            bind_group_layout: buffers.bind_group_layout,
            bind_group: buffers.bind_groupprojection,
            texture_bind_group_layout: buffers.texture_bind_group_layout,
            depth_buffer: buffers.depth_buffer,
            render_pipeline,
            transparent_pipeline,
            config,
            size_buffer,
            size_bind_group,
            ui_uniform_buffer,
            ui_bind_group,
        }
    }

    // --- Приватные helper'ы ---

    async fn request_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface<'_>) -> wgpu::Adapter {
        let adapter = instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(surface),
            ..Default::default()
        }).await.unwrap();
        println!("{}", adapter.get_info().name);
        adapter
    }

    async fn request_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
        adapter
            .request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap()
    }

    fn pick_format(surface: &wgpu::Surface, adapter: &wgpu::Adapter) -> wgpu::TextureFormat {
        surface.get_capabilities(adapter).formats[0]
    }

    fn load_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        let code = include_str!("shaders.wgsl");
        device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(code.into()),
        })
    }

    fn create_single_bind_group_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    fn create_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buffer: &wgpu::Buffer, label: &str) -> wgpu::BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn create_size_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let size = Size { map_size: 1.0 };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Size Buffer"),
            contents: bytemuck::bytes_of(&size),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }

    fn create_ui_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let ui_uniforms = UiUniforms { size: 1.0, _padding: [0.0; 3] };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI_Buffer"),
            contents: bytemuck::cast_slice(&[ui_uniforms]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        depth_stencil: &wgpu::DepthStencilState,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                buffers: &[vertex_buffer_layout()],
                module: shader,
                entry_point: "vs_main",
            },
            fragment: Some(FragmentState {
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                module: shader,
                entry_point: "fs_main",
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: Default::default(),
            multiview: None,
        })
    }

    fn create_transparent_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        depth_stencil: &wgpu::DepthStencilState,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Transparent Pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                buffers: &[vertex_buffer_layout()],
                module: shader,
                entry_point: "vs_main",
            },
            fragment: Some(FragmentState {
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                module: shader,
                entry_point: "fs_main",
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }
}

// ========================================================================
//  Вспомогательные функции на уровне модуля
// ========================================================================

fn vertex_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: VertexFormat::Float32x2,
            },
        ],
    }
}

fn surface_config(format: wgpu::TextureFormat, width: u32, height: u32) -> SurfaceConfiguration {
    SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: PresentMode::Fifo,
        alpha_mode: CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}
