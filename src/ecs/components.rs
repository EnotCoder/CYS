use specs::{Component, VecStorage};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Transform {
    pub position: [f32; 3],
}

impl Component for Transform {
    type Storage = VecStorage<Self>;
}

// SpriteComponent
#[derive(Debug)]
pub struct SpriteComponent {
    pub texture_path: String,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub scale: f32,
    pub alpha: f32,
}

impl Component for SpriteComponent {
    type Storage = VecStorage<Self>;
}

// GroupComponent 
#[derive(Debug)]
pub struct GroupComponent {
    pub group_id: u32,
}

impl Component for GroupComponent {
    type Storage = VecStorage<Self>;
}

// GroupInfo
#[derive(Debug, Clone)]
pub struct GroupInfoResource {
    pub groups: HashMap<u32, GroupInfo>,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub entities: Vec<specs::Entity>,
    pub width: i32,
    pub height: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub is_carpet: bool,
}

#[derive(Debug)]
pub struct Rotation {
    pub rotation: [f32; 3],
}

impl Component for Rotation {
    type Storage = VecStorage<Self>;
}