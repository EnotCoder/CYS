use std::sync::OnceLock;
use wgpu::util::DeviceExt;
use crate::texture::Texture;
use crate::Vertex;

fn shared_texture_layout(device: &wgpu::Device) -> &wgpu::BindGroupLayout {
    static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        })
    })
}

pub struct Sprite {
    pub texture_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub index_format: wgpu::IndexFormat,
    pub last_uniform_raw: Option<[u8; 32]>,
}

impl Sprite {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_path: &str,
        texture_frame: [i32; 2],
        texture_count: [i32; 2],
        scale: f32,
    ) -> Self {
        let sprite_x = texture_frame[0];
        let sprite_y = texture_frame[1];

        let atlas_width = texture_count[0] as f32;
        let atlas_height = texture_count[1] as f32;
        
        let tile_w = 1.0 / atlas_width;
        let tile_h = 1.0 / atlas_height;
        
        let eps = crate::constants::TEXEL_EPSILON;
        let left   = sprite_x as f32 * tile_w + eps;
        let right  = (sprite_x as f32 + 1.0) * tile_w - eps;
        let top    = sprite_y as f32 * tile_h + eps;
        let bottom = (sprite_y as f32 + 1.0) * tile_h - eps;
        
        let hs = scale * 0.5;
        let vertices: Vec<Vertex> = vec![
            Vertex { position: [-hs, hs, 0.0], tex_coord: [left, top] },
            Vertex { position: [-hs, -hs, 0.0], tex_coord: [left, bottom] },
            Vertex { position: [hs, -hs, 0.0], tex_coord: [right, bottom] },
            Vertex { position: [hs, hs, 0.0], tex_coord: [right, top] }
        ];
        let indices: Vec<u16> = crate::constants::QUAD_INDICES.to_vec();
        let index_count = indices.len() as u32;

        let texture = Texture::from_path(device, queue, texture_path, "sprite_texture");
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: shared_texture_layout(device),
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
        });
        
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

        Self {
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            index_count,
            index_format: wgpu::IndexFormat::Uint16,
            last_uniform_raw: None,
        }
    }

    pub fn from_texture(
        device: &wgpu::Device,
        texture: &Texture,
        label: &str,
        world_w: f32,
        world_h: f32,
    ) -> Self {
        let hw = world_w / 2.0;
        let hh = world_h / 2.0;
        let eps = crate::constants::TEXEL_EPSILON;
        let inv_eps = 1.0 - eps;
        let vertices: Vec<Vertex> = vec![
            Vertex { position: [-hw, hh, 0.0], tex_coord: [eps, eps] },
            Vertex { position: [-hw, -hh, 0.0], tex_coord: [eps, inv_eps] },
            Vertex { position: [hw, -hh, 0.0], tex_coord: [inv_eps, inv_eps] },
            Vertex { position: [hw, hh, 0.0], tex_coord: [inv_eps, eps] },
        ];
        let indices: Vec<u16> = crate::constants::QUAD_INDICES.to_vec();
        let index_count = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Vertex Buffer: {}", label)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Index Buffer: {}", label)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: shared_texture_layout(device),
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
        });

        Self {
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            index_count,
            index_format: wgpu::IndexFormat::Uint16,
            last_uniform_raw: None,
        }
    }
}