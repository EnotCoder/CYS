use wgpu::{util::DeviceExt, *};
use winit::dpi::PhysicalSize;

//triangle info
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

//transform class
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub translation: [f32; 4],
    pub rotation: [f32; 4],
    pub _padding: [f32; 3],
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
    
    pub fn resize(&mut self, device: &wgpu::Device, new_size: PhysicalSize<u32>) {
        let new = Self::new(device, new_size);
        self._texture = new._texture;
        self.view = new.view;
    }
}


pub struct Buffers{
    pub uniform_buffer: wgpu::Buffer,
    pub depth_buffer: DepthBuffer,
    pub depth_stencil: wgpu::DepthStencilState,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub bind_groupprojection: wgpu::BindGroup,
}

pub fn init_buffers(
    window_size: PhysicalSize<u32>,
    translation: [f32;4],
    rotation: [f32;4],
    device: &wgpu::Device,
) -> Buffers{
    let uniforms = Uniforms { 
        translation,
        rotation,
        _padding: [0.0; 3],
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
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, // Доступен только в вершинном шейдере
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

    Buffers {
        uniform_buffer,
        depth_buffer,
        depth_stencil,
        bind_group_layout,
        texture_bind_group_layout,
        bind_groupprojection,
    }
}