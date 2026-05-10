use std::fs::File;
use std::io::{BufRead, BufReader};
use wgpu::{util::DeviceExt, *};

use crate::Vertex;

pub struct Model_obj{
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn load_obj_simple(path: &str) -> Result<Model_obj, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut positions = Vec::new();
    let mut face_indices = Vec::new();
    let mut current_color = [1.0, 1.0, 1.0];
    
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
            "usemtl" => {  // Материал/цвет
                if parts.len() >= 2 && parts[1] == "Color" {
                    // Парсим цвет из строки "Color 1.0 0.0 1.0"
                    if parts.len() >= 5 {
                        let r = parts[2].parse::<f32>().unwrap_or(1.0);
                        let g = parts[3].parse::<f32>().unwrap_or(1.0);
                        let b = parts[4].parse::<f32>().unwrap_or(1.0);
                        current_color = [r, g, b];
                        println!("Changed color to: RGB({}, {}, {})", r, g, b);
                    }
                } else if parts.len() >= 2 && parts[1] == "Color" {
                    // Альтернативный парсинг (если цвет указан как "Color 1.0 0.0 1.0")
                    let r = parts[2].parse::<f32>().unwrap_or(1.0);
                    let g = parts[3].parse::<f32>().unwrap_or(1.0);
                    let b = parts[4].parse::<f32>().unwrap_or(1.0);
                    current_color = [r, g, b];
                    println!("Changed color to: RGB({}, {}, {})", r, g, b);
                }
            }
            "f" => {  // Грань
                for i in 1..parts.len() {
                    let face_part = parts[i];
                    let idx: usize = face_part.split('/').next().unwrap_or("0").parse().unwrap_or(0);
                    face_indices.push((idx - 1, current_color));  // Сохраняем индекс и цвет
                }
            }
            _ => {}
        }
    }
    
    // Создаём вершины с правильными цветами
    let mut vertices = Vec::new();
    for (idx, color) in face_indices {
        let pos = positions[idx];
        vertices.push(Vertex {
            position: [pos[0], pos[1], pos[2]],
            color: color,  // Используем цвет из материала
        });
    }
    
    let indices: Vec<u32> = (0..vertices.len() as u32).collect();
    
    Ok(Model_obj { vertices, indices })
}

pub struct ModelInstance {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub translation: [f32; 4],
    pub rotation: [f32; 4],
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl ModelInstance{
    pub fn new(
        device: &wgpu::Device,
        translation: [f32; 4],
        rotation: [f32; 4],
    )-> ModelInstance{
        let model_result = load_obj_simple("./models/table.obj");

        let (vertices, indices_u32) = match model_result {
            Ok(model) => {
                println!("Loaded model: {} vertices, {} indices", 
                        model.vertices.len(), model.indices.len());
                
                (model.vertices, model.indices)
            }
            Err(e) => {
                eprintln!("Failed to load model: {}", e);
                
                let default_vertices = vec![
                    Vertex { position: [-0.5, 0.5, 0.0], color: [0.0, 0.0, 0.6] },
                    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 0.0, 0.6] },
                    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 0.6] },
                    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
                    Vertex { position: [0.5, 0.5, 0.0], color: [0.0, 0.0, 1.0] },
                    Vertex { position: [-0.5, 0.5, 0.0], color: [0.0, 0.0, 1.0] },
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

        Self{
            vertices,
            indices,
            translation,
            rotation,
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }
}
