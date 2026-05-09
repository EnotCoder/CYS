use crate::Vertex;

pub struct Model{
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn vec_to_array<T, const N: usize>(vec: Vec<T>) -> [T; N] {
    vec.try_into().unwrap_or_else(|v: Vec<T>| {
        panic!("Expected {} elements, got {}", N, v.len())
    })
}

pub fn load_obj_simple(path: &str) -> Result<Model, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut positions = Vec::new();
    let mut face_indices = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "v" => {
                let x = parts[1].parse::<f32>().unwrap_or(0.0);
                let y = parts[2].parse::<f32>().unwrap_or(0.0);
                let z = parts[3].parse::<f32>().unwrap_or(0.0);
                positions.push([x, y, z]);
            }
            "f" => {
                for i in 1..parts.len() {
                    let face_part = parts[i];
                    let idx: usize = face_part.split('/').next().unwrap_or("0").parse().unwrap_or(0);
                    face_indices.push((idx - 1) as u32);
                }
            }
            _ => {}
        }
    }
    
    // Указываем тип явно
    let mut vertices: Vec<Vertex> = Vec::new();
    for idx in face_indices {
        let pos = positions[idx as usize];
        vertices.push(Vertex {
            position: [pos[0], pos[1], pos[2]],
            color: [pos[0].abs(), pos[1].abs(), pos[2].abs()],
        });
    }
    
    let indices: Vec<u32> = (0..vertices.len() as u32).collect();
    
    Ok(Model { vertices, indices })
}