use crate::constants::SHADER_SCALE;

pub fn ndc_to_world(mx: f32, my: f32, window_size: (f32, f32), map_size: f32) -> (f32, f32) {
    let aspect = window_size.0 / window_size.1;
    let scale = SHADER_SCALE * map_size;
    let wx = ((mx / window_size.0) * 2.0 - 1.0) * aspect / scale;
    let wy = (1.0 - (my / window_size.1) * 2.0) / scale;
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
