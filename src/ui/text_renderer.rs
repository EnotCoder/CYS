// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  TextRenderer: растеризация текста через ab_glyph в RGBA-текстуру и
//  создание из неё текстовых спрайтов. Растровые данные и спрайты кэшируются
//  по ключу «текст+размер» — повторное создание одного текста не вызывает
//  новую растеризацию.
// ========================================================================

use std::collections::HashMap;
use std::sync::Arc;
use ab_glyph::{FontRef, Font, PxScale, ScaleFont, Point};
use image::RgbaImage;
use specs::{WorldExt, Builder};
use crate::{Sprite, Texture};

pub struct TextRenderer {
    font: FontRef<'static>,
    /// Кэш растров: ключ -> (RGBA-байты, ширина, высота изображения).
    tex_cache: HashMap<String, (Vec<u8>, u32, u32)>,
}

impl TextRenderer {
    pub fn new(font_path: &str) -> Self {
        // Шрифт читается один раз и живёт всё время работы приложения.
        let font_data: &'static [u8] = Box::leak(crate::core::asset::load_bytes(font_path)
            .expect("Failed to read font file").into_boxed_slice());
        let font = FontRef::try_from_slice(font_data)
            .expect("Failed to parse font");
        Self { font, tex_cache: HashMap::new() }
    }

    /// Внутренний ключ кэша растров: объединяет текст, кегль, обводку и цвет.
    fn cache_key(text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> String {
        format!("__text__{}@{}_ol{}_c{:02x}{:02x}{:02x}", text, px_size, outline, color[0], color[1], color[2])
    }

    /// Публичный ключ для кэша спрайтов (используется destroy-функциями UI).
    pub fn sprite_cache_key(text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> u64 {
        let tk = Self::cache_key(text, px_size, outline, color);
        crate::core::util::sprite_cache_key("ui", &tk, [0, 0], [1, 1], 1.0)
    }

    /// Растеризует текст (обводка + основной цвет) в употребляемое изображение.
    /// Результат кладётся в кэш и возвращается из него.
    fn rasterize(&mut self, text: &str, px_size: f32, outline: f32, color: [u8; 3]) -> &(Vec<u8>, u32, u32) {
        let key = Self::cache_key(text, px_size, outline, color);
        if !self.tex_cache.contains_key(&key) {
            let scale = PxScale::from(px_size);
            let scaled_font = self.font.as_scaled(scale);

            // Проходим по глифам, чтобы вычислить ширину и высоту будущего изображения.
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

            // width растёт на обводку с двух сторон; высота — от базовой линии до низа.
            let ol = outline.ceil() as u32;
            total_w += ol * 2;
            if total_w == 0 { total_w = 1; }
            let baseline_row = (-min_px_top).ceil() as u32 + ol;
            let h = (baseline_row as f32 + max_px_bot + outline).ceil() as u32 + ol;
            let h = h.max(1);

            let mut image = RgbaImage::new(total_w, h);
            let ol_f = outline;

            // обводка (чёрная, рисуется первой)
            // Расширяем каждую непрозрачную точку глифа на радиус r по кругу.
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
                            // Закрашиваем чёрным все пиксели в круге радиуса r вокруг точки глифа,
                            // но не перезаписываем уже закрашенные (альфа > 200).
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
            // Достраиваем цвет поверх обводки с учётом уже залитой альфы.
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
                                // Альфа-блендинг: смешиваем цвет глифа с уже существующим пикселем.
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

    /// Мировые размеры текста при заданной ширине (высота сохраняет пропорции растров).
    pub fn text_world_size(
        &mut self,
        text: &str,
        font_size: f32,
        world_width: f32,
        outline: f32,
        color: [u8; 3],
    ) -> (f32, f32) {
        let rasterized = self.rasterize(text, font_size, outline, color);
        let (_, tw, th) = rasterized;
        let aspect = *tw as f32 / *th as f32;
        (world_width, world_width / aspect)
    }

    /// Создаёт текстовый спрайт на слое Z_UI с шириной world_width.
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
        self.add_text_z(ecs, device, queue, text, font_size, x, y, world_width, outline, color, crate::core::constants::Z_UI)
    }

    /// Инкрементальная установка текста: если текст не изменился — спрайт не трогается,
    /// иначе обновляется текстура на существующем спрайте или создаётся новый.
    /// Позиция обновляется всегда. Возвращает (сущность, ключ кэша).
    pub fn set_text(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        entity: Option<specs::Entity>,
        key: Option<u64>,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        world_width: f32,
        outline: f32,
        color: [u8; 3],
    ) -> (Option<specs::Entity>, Option<u64>) {
        let new_key = Self::sprite_cache_key(text, font_size, outline, color);

        // Позиция применяется всегда, даже если текст не менялся.
        if let Some(e) = entity {
            ecs.update_transform_position(e, x, y);
        }

        // Текст не изменился — существующий спрайт можно переиспользовать.
        if key == Some(new_key) {
            return (entity, key);
        }

        // Старый растровый ключ освобождаем из кэша спрайтов.
        if let Some(old) = key {
            ecs.sprite_cache.remove(&old);
        }

        let (rgba, tw, th) = {
            let r = self.rasterize(text, font_size, outline, color);
            (r.0.clone(), r.1, r.2)
        };
        let aspect = tw as f32 / th as f32;
        let world_h = if aspect > 0.0 { world_width / aspect } else { world_width };

        // Создаём текстуру и спрайт из растра, кладём спрайт в кэш по ключу.
        let tex = crate::Texture::from_rgba(device, queue, &rgba, tw, th, text);
        let sprite = crate::Sprite::from_texture(device, &tex, text, world_width, world_h);
        ecs.sprite_cache.insert(new_key, sprite);

        let text_key = Self::cache_key(text, font_size, outline, color);
        match entity {
            // Сущность уже есть — только меняем её текстуру на новый текст.
            Some(e) => {
                ecs.update_sprite_texture(e, &text_key);
                (Some(e), Some(new_key))
            }
            // Сущности нет — создаём новую с трансформом и спрайт-компонентом.
            None => {
                let e = ecs.world
                    .create_entity()
                    .with(crate::Transform {
                        position: [x, y, crate::core::constants::Z_UI],
                    })
                    .with(crate::SpriteComponent {
                        texture_path: std::sync::Arc::from(text_key.as_str()),
                        texture_frame: [0, 0],
                        texture_count: [1, 1],
                        scale: 1.0,
                        alpha: 1.0,
                        animated: false,
                        frame_paths: Vec::new(),
                        current_frame: 0,
                    })
                    .build();
                (Some(e), Some(new_key))
            }
        }
    }

    /// Создаёт текст с точной высотой world_height (мир. размер фиксируется явно).
    pub fn add_text_fixed(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        world_width: f32,
        world_height: f32,
        outline: f32,
        color: [u8; 3],
    ) -> specs::Entity {
        let (rgba, tw, th) = self.rasterize(text, font_size, outline, color).clone();

        let tex = Texture::from_rgba(device, queue, &rgba, tw, th, text);
        let sprite = Sprite::from_texture(device, &tex, text, world_width, world_height);

        let text_key = Self::cache_key(text, font_size, outline, color);
        let skey = crate::core::util::sprite_cache_key("ui", &text_key, [0, 0], [1, 1], 1.0);
        ecs.sprite_cache.insert(skey, sprite);

        ecs.world
            .create_entity()
            .with(crate::Transform {
                position: [x, y, crate::core::constants::Z_UI],
            })
            .with(crate::SpriteComponent {
                texture_path: Arc::from(text_key.as_str()),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: 1.0,
                alpha: 1.0,
                animated: false,
                frame_paths: Vec::new(),
                current_frame: 0,
            })
            .build()
    }

    /// Создаёт текстовый спрайт на заданном слое z (Z_UI или мир/декор).
    pub fn add_text_z(
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
        z: f32,
    ) -> specs::Entity {
        let (rgba, tw, th) = self.rasterize(text, font_size, outline, color).clone();
        let aspect = tw as f32 / th as f32;
        let world_h = world_width / aspect;

        let tex = Texture::from_rgba(device, queue, &rgba, tw, th, text);
        let sprite = Sprite::from_texture(device, &tex, text, world_width, world_h);

        let text_key = Self::cache_key(text, font_size, outline, color);
        // Ключ кэша различает слой UI и слой декора (иначе спрайты бы коллизировали).
        let layer_prefix = if (z - crate::core::constants::Z_UI).abs() < 0.001 { "ui" } else { "decor" };
        let skey = crate::core::util::sprite_cache_key(layer_prefix, &text_key, [0, 0], [1, 1], 1.0);
        ecs.sprite_cache.insert(skey, sprite);

        ecs.world
            .create_entity()
            .with(crate::Transform {
                position: [x, y, z],
            })
            .with(crate::SpriteComponent {
                texture_path: Arc::from(text_key.as_str()),
                texture_frame: [0, 0],
                texture_count: [1, 1],
                scale: 1.0,
                alpha: 1.0,
                animated: false,
                frame_paths: Vec::new(),
                current_frame: 0,
            })
            .build()
    }
}
