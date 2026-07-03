use crate::Sprite;
use crate::ecs::SpriteRenderData;
use crate::Uniforms;
use crate::util;
use std::collections::HashMap;

// ========================================================================
//  render: Главная функция рендера. Рисует слои в правильном порядке.
//
//  Порядок слоёв:
//    1. map        (z=0.0)       — первый, очищает экран и depth buffer
//    2. transparent (z=1.0-2.0)  — carpet + decor + npc + cursor (слиты в 1 pass)
//    3. ui         (z=3.0)       — UI (использует отдельный ui_bind_group)
//    4. user_cursor (z=4.0)     — кастомный курсор поверх всего
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
    decor_sprites: &[SpriteRenderData],
    npc_sprites: &[SpriteRenderData],
    cursor_sprites: &[SpriteRenderData],
    ui_sprites: &[SpriteRenderData],
    user_cursor_sprites: &[SpriteRenderData],
    size_bind_group: &wgpu::BindGroup,
    ui_bind_group: &wgpu::BindGroup,
    sprite_cache: &mut HashMap<u64, Sprite>,
    dynamic_uniform_buffer: &wgpu::Buffer,
    dynamic_bind_group: &wgpu::BindGroup,
    dynamic_alignment: u64,
) {
    let current = surface.get_current_texture();
    let frame = match current {
        wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
        wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
        _ => return,
    };
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    let mut buf_offset: u64 = 0;
    render_group(device, queue, render_pipeline, map_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "map", true,
        dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);

    let transparent_count = carpet_sprites.len() + decor_sprites.len() + npc_sprites.len() + cursor_sprites.len();
    if transparent_count > 0 {
        let mut transparent = Vec::with_capacity(transparent_count);
        transparent.extend_from_slice(carpet_sprites);
        transparent.extend_from_slice(decor_sprites);
        transparent.extend_from_slice(npc_sprites);
        transparent.extend_from_slice(cursor_sprites);
        render_group(device, queue, transparent_pipeline, &transparent, depth_view, sprite_cache,
            &mut encoder, &view, size_bind_group, "transparent", false,
            dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);
    }

    render_group(device, queue, transparent_pipeline, ui_sprites, depth_view, sprite_cache,
        &mut encoder, &view, ui_bind_group, "ui", false,
        dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);

    render_group(device, queue, transparent_pipeline, user_cursor_sprites, depth_view, sprite_cache,
        &mut encoder, &view, ui_bind_group, "user_cursor", false,
        dynamic_uniform_buffer, dynamic_bind_group, dynamic_alignment, &mut buf_offset);

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
    if sprites.is_empty() {
        return;
    }

    let color_load = if clear_color {
        wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
    } else {
        wgpu::LoadOp::Load
    };
    let depth_load = if clear_color {
        wgpu::LoadOp::Clear(crate::constants::DEPTH_CLEAR)
    } else {
        wgpu::LoadOp::Load
    };

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

    let mut keys: Vec<u64> = Vec::with_capacity(sprites.len());
    for data in sprites {
        let key = util::sprite_cache_key(
            key_prefix,
            data.position[0],
            data.position[1],
            &data.texture_path,
            data.texture_frame,
            data.texture_count,
            data.scale,
        );

        if !sprite_cache.contains_key(&key) {
            let new_sprite = Sprite::new(device, queue, &data.texture_path,
                data.texture_frame, data.texture_count, data.scale);
            sprite_cache.insert(key, new_sprite);
        }
        keys.push(key);
    }

    // Build flat uniforms array: sprites.len() * alignment bytes
    let uniform_size = std::mem::size_of::<Uniforms>();
    let write_offset = *buf_offset;
    let mut uniforms_raw: Vec<u8> = Vec::with_capacity(sprites.len() * dynamic_alignment as usize);
    for (i, (data, key)) in sprites.iter().zip(keys.iter()).enumerate() {
        let sprite = sprite_cache.get_mut(key).expect("Sprite must exist in cache");

        let uniforms = Uniforms {
            translation: [data.position[0], data.position[1], data.position[2], data.alpha],
            rotation: [data.rotation[0], data.rotation[1], data.rotation[2], 1.0],
        };
        let raw: &[u8] = bytemuck::bytes_of(&uniforms);
        let needs_update = match sprite.last_uniform_raw {
            None => true,
            Some(last) => &last[..] != raw,
        };
        if needs_update {
            sprite.last_uniform_raw = Some(raw.try_into().unwrap());
        }

        uniforms_raw.extend_from_slice(raw);
        let pad = dynamic_alignment as usize - uniform_size;
        uniforms_raw.extend(std::iter::repeat(0u8).take(pad));
        let dynamic_offset = write_offset + i as u64 * dynamic_alignment;

        render_pass.set_bind_group(0, dynamic_bind_group, &[dynamic_offset as u32]);
        render_pass.set_bind_group(1, &sprite.texture_bind_group, &[]);
        render_pass.set_bind_group(2, bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
        render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
    }

    if !uniforms_raw.is_empty() {
        queue.write_buffer(dynamic_uniform_buffer, write_offset, &uniforms_raw);
        *buf_offset = write_offset + uniforms_raw.len() as u64;
    }
}
