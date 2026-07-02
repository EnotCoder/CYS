use crate::Sprite;
use crate::ecs::SpriteRenderData;
use crate::Uniforms;
use crate::util;
use std::collections::HashMap;

// ========================================================================
//  render: Главная функция рендера. Рисует слои в правильном порядке.
//
//  Порядок слоёв:
//    1. map    (z=0.0)  — первый, очищает экран и depth buffer
//    2. carpet (z=1.0)  — ковры
//    3. decor  (z=1.5)  — декорации
//    4. npc    (z=1.8)  — NPC / персонажи
//    5. cursor (z=2.0)  — курсор
//    6. ui     (z=3.0)  — UI (использует отдельный ui_bind_group)
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
    size_bind_group: &wgpu::BindGroup,
    ui_bind_group: &wgpu::BindGroup,
    sprite_cache: &mut HashMap<u64, Sprite>,
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

    render_group(device, queue, render_pipeline, map_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "map", true);
    render_group(device, queue, transparent_pipeline, carpet_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "carpet", false);
    render_group(device, queue, transparent_pipeline, decor_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "decor", false);
    render_group(device, queue, transparent_pipeline, npc_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "npc", false);
    render_group(device, queue, transparent_pipeline, cursor_sprites, depth_view, sprite_cache,
        &mut encoder, &view, size_bind_group, "cursor", false);
    render_group(device, queue, transparent_pipeline, ui_sprites, depth_view, sprite_cache,
        &mut encoder, &view, ui_bind_group, "ui", false);

    queue.submit(std::iter::once(encoder.finish()));
    queue.present(frame);
}

// ========================================================================
//  render_group: Рисует группу спрайтов (один render pass).
//  clear_color = true только для карты (первый pass).
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

    for (data, key) in sprites.iter().zip(keys.iter()) {
        let sprite = sprite_cache.get(key).expect("Sprite must exist in cache");

        let uniforms = Uniforms {
            translation: [data.position[0], data.position[1], data.position[2], data.alpha],
            rotation: [data.rotation[0], data.rotation[1], data.rotation[2], 1.0],
            _padding: [0.0; 3],
        };
        queue.write_buffer(&sprite.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        render_pass.set_bind_group(0, &sprite.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &sprite.texture_bind_group, &[]);
        render_pass.set_bind_group(2, bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
        render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
    }
}
