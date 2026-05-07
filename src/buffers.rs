use wgpu::{util::DeviceExt, *};
use winit::dpi::PhysicalSize;

//transform class
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub translation: [f32; 4],
    pub projection: [f32; 16],
}

//depth buffer class
pub struct DepthBuffer {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl DepthBuffer {
    pub fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
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
        
        Self { _texture: texture, view }
    }
    
    pub fn resize(&mut self, device: &wgpu::Device, new_size: winit::dpi::PhysicalSize<u32>) {
        *self = Self::new(device, new_size);
    }
}

// Создаём матрицу перспективы
pub fn create_perspective_matrix(aspect: f32, fov: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov * 0.5).tan();
    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, far / (far - near), 1.0,
        0.0, 0.0, -far * near / (far - near), 0.0,
    ]
}

pub struct Buffers{
    pub projection: [f32; 16],
    pub uniform_buffer: wgpu::Buffer,
    pub depth_buffer: DepthBuffer,
    pub depth_stencil: wgpu::DepthStencilState,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_groupprojection: wgpu::BindGroup,
}

pub fn init_buffers(
    window_size: PhysicalSize<u32>,
    translation: [f32;4],
    device: &wgpu::Device,
) -> Buffers{
    let aspect = window_size.width as f32 / window_size.height as f32;

    let projection = create_perspective_matrix(aspect, std::f32::consts::PI / 4.0, 0.1, 100.0);

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
    let bind_groupprojection = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    let depth_buffer = DepthBuffer::new(&device, window_size);

    // Настройка depth_stencil для render pipeline
    let depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };

    Buffers {
        projection,
        uniform_buffer,
        depth_buffer,
        depth_stencil,
        bind_group_layout,
        bind_groupprojection,
    }
}