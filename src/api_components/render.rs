use crate::Sprite;
use crate::ecs::SpriteRenderData;
use crate::Uniforms;
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
    sprite_cache: &mut HashMap<String, Sprite>,
) {
    let frame = match surface.get_current_texture() {
        Ok(frame) => frame,
        Err(_) => return,
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
    frame.present();
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
    sprite_cache: &mut HashMap<String, Sprite>,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    bind_group: &wgpu::BindGroup,
    key_prefix: &str,
    clear_color: bool,
) {
    if sprites.is_empty() {
        return; // Нечего рисовать — пропускаем pass
    }

    // --- Настройка load operations ---
    let color_load = if clear_color {
        wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
    } else {
        wgpu::LoadOp::Load
    };
    let depth_load = if clear_color {
        wgpu::LoadOp::Clear(1.0) // Только первый pass очищает depth
    } else {
        wgpu::LoadOp::Load
    };

    // Создаём render pass
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
    });
    render_pass.set_pipeline(pipeline);

    // --- 1. Создаём недостающие спрайты + сохраняем ключи ---
    // (ключ уникален для комбинации layer + позиция + текстура)
    let mut keys: Vec<String> = Vec::with_capacity(sprites.len());
    for data in sprites {
        // Формируем ключ: "слой_x_y_путь_кадр_атлас"
        let frame_key = format!("{:?}_{:?}", data.texture_frame, data.texture_count);
        let key = format!(
            "{}_{}_{}_{}_{}_{}",
            key_prefix,
            data.position[0],
            data.position[1],
            data.texture_path,
            frame_key,
            data.scale,
        );

        // Если спрайта ещё нет в кеше — создаём
        if !sprite_cache.contains_key(&key) {
            let new_sprite = Sprite::new(device, queue, &data.texture_path,
                data.texture_frame, data.texture_count, data.scale);
            sprite_cache.insert(key.clone(), new_sprite);
        }
        keys.push(key);
    }

    // --- 2. Рисуем все спрайты (по immutable ссылкам из кеша) ---
    for (data, key) in sprites.iter().zip(keys.iter()) {
        let sprite = sprite_cache.get(key).expect("Sprite must exist in cache");

        // Обновляем uniform для каждой сущности (позиция в мире)
        let uniforms = Uniforms {
            translation: [data.position[0], data.position[1], data.position[2], 1.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            _padding: [0.0; 3],
        };
        queue.write_buffer(&sprite.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Bind группы: 0=uniform, 1=texture, 2=size/ui
        render_pass.set_bind_group(0, &sprite.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &sprite.texture_bind_group, &[]);
        render_pass.set_bind_group(2, bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
        render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
    }
}
