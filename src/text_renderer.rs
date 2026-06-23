use std::collections::HashMap;
use ab_glyph::{FontRef, Font, PxScale, ScaleFont, Point};
use image::RgbaImage;
use specs::{WorldExt, Builder};
use crate::{Sprite, Texture};

pub struct TextRenderer {
    font_data: Vec<u8>,
    tex_cache: HashMap<String, (Vec<u8>, u32, u32)>,
}

impl TextRenderer {
    pub fn new(font_path: &str) -> Self {
        let font_data = std::fs::read(font_path)
            .expect("Failed to read font file");
        Self { font_data, tex_cache: HashMap::new() }
    }

    fn cache_key(text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> String {
        format!("__text__{}@{}_ol{}_c{:02x}{:02x}{:02x}", text, px_size, outline, color[0], color[1], color[2])
    }

    pub fn sprite_cache_key(x: f32, y: f32, text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> String {
        let tk = Self::cache_key(text, px_size, outline, color);
        format!("ui_{}_{}_{}_[0, 0]_[1, 1]_1", x, y, tk)
    }

    fn rasterize(&mut self, text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> &(Vec<u8>, u32, u32) {
        let key = Self::cache_key(text, px_size, outline, color);
        if !self.tex_cache.contains_key(&key) {
            let font = FontRef::try_from_slice(&self.font_data)
                .expect("Failed to parse font");
            let scale = PxScale::from(px_size);
            let scaled_font = font.as_scaled(scale);

            let mut total_w = 0u32;
            let mut min_px_top = 0.0f32;
            let mut max_px_bot = 0.0f32;

            for c in text.chars() {
                let gid = scaled_font.glyph_id(c);
                total_w += scaled_font.h_advance(gid).ceil() as u32;

                let mut glyph = scaled_font.scaled_glyph(c);
                glyph.position = Point { x: 0.0, y: 0.0 };
                if let Some(outlined) = scaled_font.outline_glyph(glyph) {
                    let b = outlined.px_bounds();
                    if b.min.y < min_px_top { min_px_top = b.min.y; }
                    if b.max.y > max_px_bot { max_px_bot = b.max.y; }
                }
            }

            let ol = outline.ceil() as u32;
            total_w += ol * 2;
            if total_w == 0 { total_w = 1; }
            let baseline_row = (-min_px_top).ceil() as u32 + ol;
            let h = (baseline_row as f32 + max_px_bot + outline).ceil() as u32 + ol;
            let h = h.max(1);

            let mut image = RgbaImage::new(total_w, h);
            let ol_f = outline;

            // обводка (чёрная, рисуется первой)
            if outline > 0.0 {
                let r = outline as i32;
                let mut x_cursor = 0f32;
                for c in text.chars() {
                    let gid = scaled_font.glyph_id(c);
                    let mut glyph = scaled_font.scaled_glyph(c);
                    glyph.position = Point { x: x_cursor + ol_f, y: 0.0 };
                    if let Some(outlined) = scaled_font.outline_glyph(glyph) {
                        let b = outlined.px_bounds();
                        let y_off = baseline_row as i32 + b.min.y as i32;
                        outlined.draw(|gx, gy, cover| {
                            if cover <= 0.0 { return; }
                            let px = (b.min.x + gx as f32) as i32;
                            let py = (y_off + gy as i32) as i32;
                            for dy in -r..=r {
                                for dx in -r..=r {
                                    if dx * dx + dy * dy <= r * r {
                                        let sx = px + dx;
                                        let sy = py + dy;
                                        if sx >= 0 && sx < total_w as i32 && sy >= 0 && sy < h as i32 {
                                            let pix = image.get_pixel_mut(sx as u32, sy as u32);
                                            if pix[3] < 200 {
                                                pix[0] = 0; pix[1] = 0; pix[2] = 0; pix[3] = 255;
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                    x_cursor += scaled_font.h_advance(gid);
                }
            }

            // основной текст (заданный цвет)
            {
                let mut x_cursor = 0f32;
                for c in text.chars() {
                    let gid = scaled_font.glyph_id(c);
                    let mut glyph = scaled_font.scaled_glyph(c);
                    glyph.position = Point { x: x_cursor + ol_f, y: 0.0 };
                    if let Some(outlined) = scaled_font.outline_glyph(glyph) {
                        let b = outlined.px_bounds();
                        let y_off = baseline_row as i32 + b.min.y as i32;
                        outlined.draw(|gx, gy, cover| {
                            let px = (b.min.x + gx as f32) as u32;
                            let py = (y_off + gy as i32) as u32;
                            if px < total_w && py < h && cover > 0.0 {
                                let fg_a = cover.min(1.0);
                                let pix = image.get_pixel_mut(px, py);
                                let bg_a = pix[3] as f32 / 255.0;
                                let out_a = fg_a + bg_a * (1.0 - fg_a);
                                let out_v = fg_a / out_a;
                                pix[0] = (out_v * color[0] as f32) as u8;
                                pix[1] = (out_v * color[1] as f32) as u8;
                                pix[2] = (out_v * color[2] as f32) as u8;
                                pix[3] = (out_a * 255.0) as u8;
                            }
                        });
                    }
                    x_cursor += scaled_font.h_advance(gid);
                }
            }

            self.tex_cache.insert(key.clone(), (image.into_raw(), total_w, h));
        }
        &self.tex_cache[&key]
    }

    pub fn add_text(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        world_width: f32,
        outline: f32,
        color: [u8; 3],
    ) -> specs::Entity {
        let (rgba, tw, th) = self.rasterize(text, font_size, outline, color).clone();
        let aspect = tw as f32 / th as f32;
        let world_h = world_width / aspect;

        let tex = Texture::from_rgba(device, queue, &rgba, tw, th, text);
        let sprite = Sprite::from_texture(device, &tex, text, world_width, world_h);

        let skey = Self::sprite_cache_key(x, y, text, font_size, outline, color);
        ecs.sprite_cache.insert(skey, sprite);

        let text_key = Self::cache_key(text, font_size, outline, color);
        ecs.world
            .create_entity()
            .with(crate::Transform {
                position: [x, y, crate::constants::Z_UI],
            })
            .with(crate::SpriteComponent {
                texture_path: text_key,
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: 1.0,
            })
            .build()
    }
}
