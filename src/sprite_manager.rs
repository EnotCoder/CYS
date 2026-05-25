use wgpu::{util::DeviceExt, *};
use crate::texture::Texture;
use crate::Vertex;

pub struct Sprite {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub texture_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl Sprite {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_path: &str,
    ) -> Self {
        let vertices: Vec<Vertex> = Vec::new();
        let indices: Vec<u16> = Vec::new();
        
        // Загружаем текстуру
        let texture = Texture::from_path(device, queue, texture_path, "sprite_texture")
            .expect("Failed to load texture");
        
        let texture_bind_group = Self::create_texture_bind_group(device, &texture);
        
        // Создаём пустые буферы (нужно указать тип)
        let empty_vertices: &[Vertex] = &[];
        let empty_indices: &[u16] = &[];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Batch Vertex Buffer"),
            contents: bytemuck::cast_slice(empty_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Batch Index Buffer"),
            contents: bytemuck::cast_slice(empty_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        
        Self {
            vertices,
            indices,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            index_count: 0,
        }
    }
    
    pub fn add_sprite(&mut self, x: f32, y: f32, z: f32) {
        let base_index = self.vertices.len() as u16;
        
        self.vertices.push(Vertex { 
            position: [-0.5 + x, 0.5 + y, z], 
            tex_coord: [0.0, 0.0], 
        });
        self.vertices.push(Vertex { 
            position: [-0.5 + x, -0.5 + y, z], 
            tex_coord: [0.0, 1.0], 
        });
        self.vertices.push(Vertex { 
            position: [0.5 + x, -0.5 + y, z], 
            tex_coord: [1.0, 1.0], 
        });
        self.vertices.push(Vertex { 
            position: [0.5 + x, 0.5 + y, z], 
            tex_coord: [1.0, 0.0], 
        });
        
        self.indices.push(base_index);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);
        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);
    }
    
    pub fn build_buffers(&mut self, device: &wgpu::Device) {
        if self.vertices.is_empty() {
            return;
        }
        
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Batch Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Batch Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        self.index_count = self.indices.len() as u32;
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