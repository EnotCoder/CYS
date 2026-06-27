use specs::{World, Entity, Builder, WorldExt};
use crate::ecs::components::{Transform, SpriteComponent};

pub fn create_sprite(
    world: &mut World, x: f32, y: f32, z: f32,
    texture_path: &str, frame: [i32; 2], count: [i32; 2],
    scale: f32, alpha: f32,
) -> Entity {
    world
        .create_entity()
        .with(Transform { position: [x, y, z] })
        .with(SpriteComponent {
            texture_path: texture_path.to_string(),
            texture_frame: frame,
            texture_count: count,
            scale,
            alpha,
            animated: false,
            frame_paths: Vec::new(),
            current_frame: 0,
        })
        .build()
}


