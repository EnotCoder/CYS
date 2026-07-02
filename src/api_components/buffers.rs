use winit::dpi::PhysicalSize;

//triangle info
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

//transform class — matches WGSL Uniforms (2x vec4)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub translation: [f32; 4],
    pub rotation: [f32; 4],
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
    pub dynamic_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_buffer: DepthBuffer,
    pub depth_stencil: wgpu::DepthStencilState,
    pub transparent_depth_stencil: wgpu::DepthStencilState,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

pub fn init_buffers(
    window_size: PhysicalSize<u32>,
    device: &wgpu::Device,
) -> Buffers{
    // Bind group layout for dynamic storage buffer (group 0)
    // Storage instead of Uniform because max_uniform_buffer_binding_size = 65536
    let dynamic_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Dynamic Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                },
                count: None,
            },
        ],
    });

    // Создаём depth texture
    let depth_buffer = DepthBuffer::new(&device, window_size);

    // Настройка depth_stencil для render pipeline
    let depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };

    let transparent_depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Always),
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
        dynamic_bind_group_layout,
        transparent_depth_stencil,
        depth_buffer,
        depth_stencil,
        texture_bind_group_layout,
    }
}