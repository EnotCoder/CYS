use std::hash::{Hash, Hasher};
use crate::constants::SHADER_SCALE;

pub fn sprite_cache_key(layer: &str, x: f32, y: f32, path: &str, frame: [i32; 2], atlas: [i32; 2], scale: f32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    layer.hash(&mut hasher);
    x.to_bits().hash(&mut hasher);
    y.to_bits().hash(&mut hasher);
    path.hash(&mut hasher);
    frame.hash(&mut hasher);
    atlas.hash(&mut hasher);
    scale.to_bits().hash(&mut hasher);
    hasher.finish()
}

pub fn ndc_to_world(mx: f32, my: f32, window_size: (f32, f32), map_size: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let aspect = window_size.0 / window_size.1;
    let scale = SHADER_SCALE * map_size;
    let wx = ((mx / window_size.0) * 2.0 - 1.0) * aspect / scale + cam_x;
    let wy = (1.0 - (my / window_size.1) * 2.0) / scale + cam_y;
    (wx, wy)
}

#[allow(dead_code)]
pub fn grid_pos_from_world(wx: f32, wy: f32) -> (i32, i32) {
    let col = (wx - crate::constants::SLOT_BAR_X + crate::constants::TILE_HALF) as i32;
    (col, (wy - crate::constants::SLOT_BAR_Y + crate::constants::TILE_HALF) as i32)
}

pub fn inventory_index(row: i32, col: i32) -> i32 {
    (crate::constants::INVENTORY_ROWS - 1 - row) * crate::constants::INVENTORY_COLS + col
}
