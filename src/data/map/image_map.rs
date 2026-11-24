// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Карта как готовое изображение (assets/map/*.png)
// ========================================================================
//  Игрок рисует карту в Tiled и экспортирует единый PNG 16 px на клетку.
//  Один спрайт на всю карту заменяет тысячу тайловых сущностей. Игровая
//  логика (проходимость, стены, пол, улица) восстанавливается сопоставлением
//  каждого 16×16 блока с кадрами существующих атласов: пиксели, непрозрачные
//  в кадре атласа, обязаны совпасть с пикселями картинки. Приоритет более
//  высокого слоя (стена > пол > дорога > трава) выбирает верхний тайл.

use crate::core::constants::{DOOR_TOKEN, DOOR_X_MAX, DOOR_X_MIN, DOOR_Y_MAX, DOOR_Y_MIN};
use std::sync::OnceLock;

const TILE: usize = 16;

// Порядок = приоритет при наложении слоёв (последний выигрывает).
const ATLASES: [(&str, &str, u8); 5] = [
    ("grass", "assets/tex/map/grass.png", 0),
    ("road", "assets/tex/map/stone_road.png", 1),
    ("floor", "assets/tex/map/floor.png", 2),
    ("wall", "assets/tex/map/wall.png", 3),
    ("water_wall", "assets/tex/map/water_shop_wall.png", 4),
];

#[derive(Clone, Debug)]
pub struct ImageCell {
    pub token: &'static str,
}

#[derive(Clone, Debug)]
pub struct ImageMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<ImageCell>>,
}

impl ImageMap {
    #[cfg(test)]
    pub fn token_at(&self, col: usize, row: usize) -> Option<&'static str> {
        self.cells.get(row).and_then(|line| line.get(col)).map(|c| c.token)
    }
}

struct Frame {
    pixels: Vec<[u8; 4]>,
    opaque: Vec<usize>,
    token: &'static str,
    priority: u8,
}

fn token_for_atlas(name: &str) -> &'static str {
    match name {
        "grass" | "road" => ".",
        "floor" => "0",
        "wall" | "water_wall" => "W",
        _ => ".",
    }
}

fn load_frames() -> Result<Vec<Frame>, String> {
    let mut frames = Vec::new();
    for (name, path, priority) in ATLASES {
        let bytes = crate::core::asset::load_bytes(path)
            .map_err(|e| format!("atlas {path}: {e}"))?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| format!("decode {path}: {e}"))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        if w % TILE as u32 != 0 || h % TILE as u32 != 0 {
            return Err(format!("атлас {path} не кратен {TILE}"));
        }
        let raw = img.as_raw();
        for fy in 0..(h as usize / TILE) {
            for fx in 0..(w as usize / TILE) {
                let base = ((fy * TILE) * w as usize + fx * TILE) * 4;
                let src = |x: usize, y: usize| -> [u8; 4] {
                    let i = base + (y * w as usize + x) * 4;
                    [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]]
                };
                for flip in 0..8u8 {
                    let mut pixels = Vec::with_capacity(TILE * TILE);
                    for i in 0..TILE * TILE {
                        let (mut x, mut y) = (i % TILE, i / TILE);
                        if flip & 4 != 0 {
                            std::mem::swap(&mut x, &mut y);
                        }
                        if flip & 2 != 0 {
                            y = TILE - 1 - y;
                        }
                        if flip & 1 != 0 {
                            x = TILE - 1 - x;
                        }
                        pixels.push(src(x, y));
                    }
                    let opaque: Vec<usize> = pixels
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p[3] == 255)
                        .map(|(i, _)| i)
                        .collect();
                    if opaque.is_empty() {
                        continue;
                    }
                    frames.push(Frame {
                        pixels,
                        opaque,
                        token: token_for_atlas(name),
                        priority,
                    });
                }
            }
        }
    }
    Ok(frames)
}

fn classify_tile(tile: &[[u8; 4]], frames: &[Frame]) -> &'static str {
    let mut best: Option<&Frame> = None;
    for frame in frames {
        if best.map_or(false, |b| frame.priority <= b.priority) {
            continue;
        }
        if frame.opaque.iter().all(|&i| tile[i] == frame.pixels[i]) {
            best = Some(frame);
        }
    }
    best.map_or(".", |f| f.token)
}

/// Разбирает картинку карты в сетку категорий. Вызывается один раз, результат
/// переиспользуется загрузкой уровня и pathfinding'ом покупателей.
pub fn parse_map_image(bytes: &[u8]) -> Result<ImageMap, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("decode map: {e}"))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || w % TILE as u32 != 0 || h % TILE as u32 != 0 {
        return Err(format!("размер карты {w}×{h} не кратен {TILE}"));
    }
    let width = w as usize / TILE;
    let height = h as usize / TILE;
    let frames = load_frames()?;
    let raw = img.as_raw();

    let mut cells = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = Vec::with_capacity(width);
        for col in 0..width {
            let mut tile = [[0u8; 4]; TILE * TILE];
            for y in 0..TILE {
                for x in 0..TILE {
                    let i = ((row * TILE + y) * w as usize + col * TILE + x) * 4;
                    tile[y * TILE + x] = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                }
            }
            line.push(ImageCell {
                token: classify_tile(&tile, &frames),
            });
        }
        cells.push(line);
    }

    let mut map = ImageMap {
        width,
        height,
        cells,
    };
    for y in DOOR_Y_MIN..=DOOR_Y_MAX {
        for x in DOOR_X_MIN..=DOOR_X_MAX {
            let col = x - crate::core::constants::WORLD_OFFSET_X as i32;
            let row = crate::core::constants::WORLD_OFFSET_Y as i32 - y;
            if col >= 0 && (col as usize) < map.width && row >= 0 && (row as usize) < map.height {
                map.cells[row as usize][col as usize].token = DOOR_TOKEN;
            }
        }
    }
    Ok(map)
}

static MAP_CACHE: OnceLock<ImageMap> = OnceLock::new();

pub fn cached_map() -> &'static ImageMap {
    MAP_CACHE.get_or_init(|| {
        let src = crate::core::asset::load_bytes(crate::core::constants::MAP_FILE)
            .expect("assets/map/map.png not found");
        parse_map_image(&src).unwrap_or_else(|e| panic!("invalid map image: {e}"))
    })
}
