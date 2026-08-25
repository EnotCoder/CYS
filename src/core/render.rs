// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use crate::Sprite;
use crate::Texture;
use crate::ecs::SpriteRenderData;
use crate::Uniforms;
use crate::core::util;
use std::collections::HashMap;

// ========================================================================
//  render: Главная функция рендера. Рисует слои в правильном порядке.
//
//  Порядок слоёв:
//    1. map        (z=0.0)       — первый, очищает экран и depth buffer
//    2. transparent (z=1.0-2.0)  — carpet + light + decor + npc + cursor (слиты в 1 pass)
//    3. ui         (z=3.0)       — UI (использует отдельный ui_bind_group)
// ========================================================================
pub fn render(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_pipeline: &wgpu::RenderPipeline,
    transparent_pipeline: &wgpu::RenderPipeline,
    depth_view: &wgpu::TextureView,
    map_sprites: &[SpriteRenderData],
    carpet_sprites: &[SpriteRenderData],
    light_sprites: &[SpriteRenderData],
    decor_sprites: &[SpriteRenderData],
    npc_sprites: &[SpriteRenderData],
    cursor_sprites: &[SpriteRenderData],
    ui_sprites: &[SpriteRenderData],
    size_bind_group: &wgpu::BindGroup,
    ui_bind_group: &wgpu::BindGroup,
    sprite_cache: &mut HashMap<u64, Sprite>,
    texture_cache: &mut HashMap<String, Texture>,
    dynamic_uniform_buffer: &wgpu::Buffer,
    dynamic_bind_group: &wgpu::BindGroup,
    dynamic_alignment: u64,
    light_data: &[crate::core::buffers::LightData],
    light_buffer: &wgpu::Buffer,
) {
    // Пишем данные о свете в буфер перед отрисовкой
    if !light_data.is_empty() {
        queue.write_buffer(light_buffer, 0, bytemuck::cast_slice(light_data));
    }

    // Получаем текущий кадр поверхности. Suboptimal тоже показываем —
    // это лишь сигнал о том, что размер окна скоро изменится.
    let current = surface.get_current_texture();
    let frame = match current {
        wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
        wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
        _ => return,
    };
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

    // CommandEncoder накапливает все команды кадра, которые потом
    // одним куском отправляются в очередь (queue.submit).
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    // buf_offset — позиция в dynamic_uniform_buffer, куда пишем uniform'ы.
    // Все вызовы render_group делят один буфер, поэтому счётчик сквозной.
    let mut buf_offset: u64 = 0;

    // Слой карты: первый проход, чистит экран и depth buffer (clear=true).
    render_group(device, queue, render_pipeline, map_sprites, depth_view, sprite_cache,
        texture_cache, &mut encoder, &view, size_bind_group, "map", true,
        dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);

    // Прозрачные слои (carpet/light/decor/npc/cursor) объединяем в один массив
    // (порядок = z-порядок), чтобы рисовать их одним проходом и одним батчем
    // записи uniform'ов.
    let transparent_count = carpet_sprites.len() + light_sprites.len() + decor_sprites.len() + npc_sprites.len() + cursor_sprites.len();
    if transparent_count > 0 {
        let mut transparent = Vec::with_capacity(transparent_count);
        transparent.extend_from_slice(carpet_sprites);
        transparent.extend_from_slice(light_sprites);
        transparent.extend_from_slice(decor_sprites);
        transparent.extend_from_slice(npc_sprites);
        transparent.extend_from_slice(cursor_sprites);
        render_group(device, queue, transparent_pipeline, &transparent, depth_view, sprite_cache,
            texture_cache, &mut encoder, &view, size_bind_group, "transparent", false,
            dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);
    }

    // UI рисуется последним поверх всего (z=3.0), со своим bind group.
    // Прозрачный пайплайн игнорирует тест глубины (depth = Always), поэтому
    // порядок отрисовки = порядку в массиве. Сортируем по z по возрастанию,
    // чтобы «подложки» (z чуть ниже) рисовались раньше текста (z выше) и
    // не перекрывали его полупрозрачным чёрным.
    let mut ui_sorted = ui_sprites.to_vec();
    ui_sorted.sort_by(|a, b| {
        a.position[2].partial_cmp(&b.position[2]).unwrap_or(std::cmp::Ordering::Equal)
    });
    render_group(device, queue, transparent_pipeline, &ui_sorted, depth_view, sprite_cache,
        texture_cache, &mut encoder, &view, ui_bind_group, "ui", false,
        dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);

    // Отправляем все команды кадра в очередь и показываем результат на экране.
    queue.submit(std::iter::once(encoder.finish()));
    queue.present(frame);
}

// ========================================================================
//  render_group: Рисует группу спрайтов (один render pass).
//  clear_color = true только для карты (первый pass).
//  Все uniform'ы записываются одним batch queue.write_buffer.
// ========================================================================
fn render_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    sprites: &[SpriteRenderData],
    depth_view: &wgpu::TextureView,
    sprite_cache: &mut HashMap<u64, Sprite>,
    texture_cache: &mut HashMap<String, Texture>,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    bind_group: &wgpu::BindGroup,
    key_prefix: &str,
    clear_color: bool,
    dynamic_uniform_buffer: &wgpu::Buffer,
    dynamic_bind_group: &wgpu::BindGroup,
    dynamic_alignment: u64,
    buf_offset: &mut u64,
) {
    // Пустую группу пропускаем — не создаём лишний render pass.
    if sprites.is_empty() {
        return;
    }

    // Первый проход (карта) очищает и цвет и глубину,
    // остальные — дописывают поверх уже нарисованного (Load).
    let color_load = if clear_color {
        wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
    } else {
        wgpu::LoadOp::Load
    };
    let depth_load = if clear_color {
        wgpu::LoadOp::Clear(crate::core::constants::DEPTH_CLEAR)
    } else {
        wgpu::LoadOp::Load
    };

    // Начинаем render pass: цветовой attachment (экран) + depth attachment.
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: color_load, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations { load: depth_load, store: wgpu::StoreOp::Store }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    });
    render_pass.set_pipeline(pipeline);

    // Первый проход по спрайтам: достаём/загружаем текстуры в кэш.
    // key — хэш из пути к текстуре + кадра + масштаба; если такой спрайт
    // уже создавался — берём из кэша и не грузим текстуру повторно.
    let mut keys: Vec<u64> = Vec::with_capacity(sprites.len());
    for data in sprites {
        let key = util::sprite_cache_key(
            key_prefix,
            &data.texture_path,
            data.texture_frame,
            data.texture_count,
            data.scale,
        );

        if !sprite_cache.contains_key(&key) {
            // Текстура кэшируется по базовому пути (без «@WxH»), чтобы
            // новые масштабы не перечитывали файл и не дублировали GPU-ресурсы.
            let (base, _) = Sprite::split_sized_path(&data.texture_path);
            let base_owned = base.to_string();
            if !texture_cache.contains_key(&base_owned) {
                let tex = Texture::from_path(device, queue, &base_owned, "texture_cache");
                texture_cache.insert(base_owned.clone(), tex);
            }
            let texture = &texture_cache[&base_owned];
            let new_sprite = Sprite::new(device, texture, &data.texture_path,
                data.texture_frame, data.texture_count, data.scale);
            sprite_cache.insert(key, new_sprite);
        }
        keys.push(key);
    }

    // Build flat uniforms array: sprites.len() * alignment bytes
    // Собираем __все__ uniform'ы группы в один плоский массив байтов.
    // Каждый блок (Uniforms) дополняем нулями до кратности alignment,
    // чтобы динамический offset указывал ровно на начало следующего блока.
    let uniform_size = std::mem::size_of::<Uniforms>();
    let write_offset = *buf_offset;
    let mut uniforms_raw: Vec<u8> = Vec::with_capacity(sprites.len() * dynamic_alignment as usize);
    for (i, (data, key)) in sprites.iter().zip(keys.iter()).enumerate() {
        let sprite = sprite_cache.get_mut(key).expect("Sprite must exist in cache");

        let uniforms = Uniforms {
            // translation.w несёт альфу (прозрачность) спрайта.
            translation: [data.position[0], data.position[1], data.position[2], data.alpha],
            // rotation.w — резервный флаг, остаётся 1.0.
            rotation: [data.rotation[0], data.rotation[1], data.rotation[2], 1.0],
        };
        let raw: &[u8] = bytemuck::bytes_of(&uniforms);
        // Обновляем кэш последних uniform'ов спрайта.
        // (позволяет later этапам понять, изменились данные или нет).
        let needs_update = match sprite.last_uniform_raw {
            None => true,
            Some(last) => &last[..] != raw,
        };
        if needs_update {
            sprite.last_uniform_raw = Some(raw.try_into().unwrap());
        }

        uniforms_raw.extend_from_slice(raw);
        // Дополняем до alignment нулями (padded slot).
        let pad = dynamic_alignment as usize - uniform_size;
        uniforms_raw.extend(std::iter::repeat(0u8).take(pad));
        let dynamic_offset = write_offset + i as u64 * dynamic_alignment;

        // Привязываем ресурсы для текущего спрайта:
        // group 0 — его uniform (по dynamic offset), 1 — текстура, 2 — Size/UI.
        render_pass.set_bind_group(0, dynamic_bind_group, &[dynamic_offset as u32]);
        render_pass.set_bind_group(1, &sprite.texture_bind_group, &[]);
        render_pass.set_bind_group(2, bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
        // Рисуем один квад спрайта (0..index_count вершин, 1 экземпляр).
        render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
    }

    // Записываем собранный массив uniform'ов одним write_buffer
    // (одна передача вместо N отдельных — и меньше CPU-нагрузки).
    if !uniforms_raw.is_empty() {
        queue.write_buffer(dynamic_uniform_buffer, write_offset, &uniforms_raw);
        *buf_offset = write_offset + uniforms_raw.len() as u64;
    }
}
