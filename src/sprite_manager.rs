use wgpu::{util::DeviceExt};
use crate::texture::Texture;
use crate::Vertex;
use crate::Uniforms;

pub struct Sprite {
    pub texture_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub index_count: u32,
    pub index_format: wgpu::IndexFormat,
    buffers_initialized: bool,
}

impl Drop for Sprite {
    fn drop(&mut self) {
        if self.buffers_initialized {
            //#[cfg(debug_assertions)]
            //println!("Sprite dropped: {} (resources will be released)", self.texture_path);
            self.buffers_initialized = false;
            // texture автоматически очистится при падении Option
        }
    }
}

impl Sprite {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_path: &str,
        texture_frame: [i32; 2],
        texture_count: [i32; 2],
    ) -> Self {
        let sprite_x = texture_frame[0];
        let sprite_y = texture_frame[1];

        let atlas_width = texture_count[0] as f32;
        let atlas_height = texture_count[1] as f32;
        
        let tile_w = 1.0 / atlas_width;
        let tile_h = 1.0 / atlas_height;
        
        // Отступ в долях texel'а — чтобы на границах атласа не было грязи
        // (ClampToEdge + небольшой inset убирает серые линии между тайлами)
        let eps = 0.001;
        let left   = sprite_x as f32 * tile_w + eps;
        let right  = (sprite_x as f32 + 1.0) * tile_w - eps;
        let top    = sprite_y as f32 * tile_h + eps;
        let bottom = (sprite_y as f32 + 1.0) * tile_h - eps;
        
        let vertices: Vec<Vertex> = vec![
            Vertex { position: [-0.5, 0.5, 0.0], tex_coord: [left, top] },
            Vertex { position: [-0.5, -0.5, 0.0], tex_coord: [left, bottom] },
            Vertex { position: [0.5, -0.5, 0.0], tex_coord: [right, bottom] },
            Vertex { position: [0.5, 0.5, 0.0], tex_coord: [right, top] }
        ];
        let indices: Vec<u16> = vec![0, 1, 2, 2, 3, 0];
        let index_count = indices.len() as u32;

        let texture = Texture::from_path(device, queue, texture_path, "sprite_texture");
        let texture_bind_group = Self::create_texture_bind_group(device, &texture);
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Vertex Buffer: {}", texture_path)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Index Buffer: {}", texture_path)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms = Uniforms { 
            translation: [0.0, 0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 0.0],
            _padding: [0.0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Uniform Buffer: {}", texture_path)),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = Self::create_uniform_bind_group(device, &uniform_buffer);

        Self {
            uniform_buffer,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            index_count,
            uniform_bind_group,
            index_format: wgpu::IndexFormat::Uint16,
            buffers_initialized: true,
        }
    }
    
    fn create_uniform_bind_group(device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        })
    }
    
    fn create_texture_bind_group(device: &wgpu::Device, texture: &Texture) -> wgpu::BindGroup {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }
}