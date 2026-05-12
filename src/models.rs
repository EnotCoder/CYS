use std::fs::File;
use std::io::{BufRead, BufReader};
use wgpu::{util::DeviceExt, *};


use crate::texture::Texture;
use crate::Uniforms;
use crate::Vertex;

pub struct Model_obj{
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn load_obj_simple(path: &str) -> Result<Model_obj, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut positions = Vec::new();
    let mut tex_coords = Vec::new();
    let mut face_indices = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "v" => {  // Вершина (позиция)
                let x = parts[1].parse::<f32>().unwrap_or(0.0);
                let y = parts[2].parse::<f32>().unwrap_or(0.0);
                let z = parts[3].parse::<f32>().unwrap_or(0.0);
                positions.push([x, y, z]);
            }
            "vt" => {  // Материал/цвет
                let u = parts[1].parse::<f32>().unwrap_or(0.0);
                let v = parts[2].parse::<f32>().unwrap_or(0.0);

                tex_coords.push([u, v]);
            }
            "f" => {  // Грань
                for i in 1..parts.len() {
                    let face_part = parts[i];
                    // indices = ["5", "3"]
                    let indices: Vec<&str> = face_part.split('/').collect();
                    
                    // indices[0] = "5"
                    // parse::<usize>() = 5
                    // 5 - 1 = 4
                    // pos_idx = 4 (индекс вершины в массиве positions)
                    let pos_idx = indices[0].parse::<usize>().unwrap_or(0) - 1;
                    
                    // Индекс текстурных координат
                    let tex_idx = if indices.len() > 1 && !indices[1].is_empty() {
                        Some(indices[1].parse::<usize>().unwrap_or(0) - 1)
                    } else {
                        None
                    };
                    
                    face_indices.push((pos_idx, tex_idx));
                }
            }
            _ => {}
        }
    }
    
    // Создаём вершины с правильными цветами
    let mut vertices = Vec::new();
    for (pos_idx, tex_idx_opt) in face_indices {
        let pos = positions[pos_idx];
        
        // Получаем UV координаты, если они есть
        let tex = match tex_idx_opt {
            Some(tex_idx) if tex_idx < tex_coords.len() => tex_coords[tex_idx],
            _ => [0.0, 0.0],  // Значение по умолчанию, если UV нет
        };
        
        vertices.push(Vertex {
            position: [pos[0], pos[1], pos[2]],
            tex_coord: tex,
        });
    }
    
    let indices: Vec<u32> = (0..vertices.len() as u32).collect();
    
    Ok(Model_obj { vertices, indices })
}

pub struct ModelInstance {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub translation: [f32; 4],
    pub translation_base: [f32; 4],
    pub rotation: [f32; 4],
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub texture_bind_group: wgpu::BindGroup,
}

impl ModelInstance{
    pub fn new(
        path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        translation: [f32; 4],
        translation_base: [f32; 4],
        rotation: [f32; 4],
        projection: [f32; 16],
        texture_path: &str,
    )-> ModelInstance{
        let model_result = load_obj_simple(path);

        let (vertices, indices_u32) = match model_result {
            Ok(model) => {
                println!("Loaded model: {} vertices, {} indices", 
                        model.vertices.len(), model.indices.len());
                
                (model.vertices, model.indices)
            }
            Err(e) => {
                eprintln!("Failed to load model: {}", e);
                
                let default_vertices = vec![
                    Vertex { position: [-0.5, 0.5, 0.0], tex_coord: [0.0, 0.0] },
                    Vertex { position: [-0.5, -0.5, 0.0], tex_coord: [0.0, 0.0] },
                    Vertex { position: [0.5, -0.5, 0.0], tex_coord: [0.0, 0.0] },
                    Vertex { position: [0.5, -0.5, 0.0], tex_coord: [0.0, 0.0] },
                    Vertex { position: [0.5, 0.5, 0.0], tex_coord: [0.0, 0.0] },
                    Vertex { position: [-0.5, 0.5, 0.0], tex_coord: [0.0, 0.0] },
                ];

                let default_indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

                (default_vertices, default_indices)
            }
        };

        let indices: Vec<u16> = indices_u32.iter().map(|&i| i as u16).collect();
        let index_count = indices.len() as u32;

        // Создаём буфер вершин
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Vertex Buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Создаём буфер индексов
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Index Buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Создаём свой uniform buffer для этой модели
        let uniforms = Uniforms { translation, rotation, projection };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Создаём bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        // Создаём bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        //texture

        let texture = Texture::from_path(device, queue, texture_path, "model_texture")
            .expect("Failed to load texture");

        let t_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
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
        

        let t_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &t_bind_group_layout,
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

        Self{
            vertices,
            indices,
            translation,
            translation_base,
            rotation,
            vertex_buffer,
            index_buffer,
            index_count,
            uniform_buffer,
            bind_group,
            texture_bind_group: t_bind_group,
        }
    }

    pub fn update_transform(&self, queue: &wgpu::Queue, projection: [f32; 16]) {
        let uniforms = Uniforms { 
            translation: self.translation, 
            rotation: self.rotation, 
            projection 
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }


}
