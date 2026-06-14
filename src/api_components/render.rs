use crate::Sprite;
use crate::ecs::SpriteRenderData;
use crate::Uniforms;
use std::collections::HashMap;

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
    cursor_sprites: &[SpriteRenderData],
    ui_sprites: &[SpriteRenderData],
    _bind_group: &wgpu::BindGroup,
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

    // render map (первый pass - очищает экран)
    render_group(
        device, queue, render_pipeline, map_sprites, 
        depth_view, sprite_cache, &mut encoder, &view, size_bind_group,
        "map",
        true,
    );
    
    // render carpets (загружает существующее изображение)
    render_group(
        device, queue, transparent_pipeline, carpet_sprites, 
        depth_view, sprite_cache, &mut encoder, &view, size_bind_group,
        "carpet",
        false,
    );
    
    // render decor
    render_group(
        device, queue, transparent_pipeline, decor_sprites, 
        depth_view, sprite_cache, &mut encoder, &view, size_bind_group,
        "decor",
        false,
    );
    
    // render cursor
    render_group(
        device, queue, transparent_pipeline, cursor_sprites, 
        depth_view, sprite_cache, &mut encoder, &view, size_bind_group,
        "cursor",
        false,
    );
    
    // render ui (использует ui_bind_group)
    render_group(
        device, queue, transparent_pipeline, ui_sprites, 
        depth_view, sprite_cache, &mut encoder, &view, ui_bind_group,
        "ui",
        false,
    );
    
    queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}

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
    // Определяем load operation для цвета
    let color_load_op = if clear_color {
        wgpu::LoadOp::Clear(wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.2,
            a: 1.0,
        })
    } else {
        wgpu::LoadOp::Load
    };
    
    // Для глубины всегда используем Load (сохраняем depth buffer между pass'ами)
    let depth_load_op = if clear_color {
        wgpu::LoadOp::Clear(1.0)  // только первый pass очищает depth
    } else {
        wgpu::LoadOp::Load
    };
    
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: color_load_op,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: depth_load_op,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    
    render_pass.set_pipeline(pipeline);
    
    // СНАЧАЛА создаём все спрайты (мутабельный borrow)
    for sprite_data in sprites {
        let key = format!(
            "{}_{}_{}_{}_{:?}_{:?}",
            key_prefix,
            sprite_data.position[0],
            sprite_data.position[1],
            sprite_data.texture_path,
            sprite_data.texture_frame,
            sprite_data.texture_count
        );
        
        if !sprite_cache.contains_key(&key) {
            let new_sprite = Sprite::new(
                device,
                queue,
                &sprite_data.texture_path,
                sprite_data.texture_frame,
                sprite_data.texture_count,
            );
            sprite_cache.insert(key.clone(), new_sprite);
        }
    }
    
    // ПОТОМ рисуем все спрайты (immutable borrow)
    for sprite_data in sprites {
        let key = format!(
            "{}_{}_{}_{}_{:?}_{:?}",
            key_prefix,
            sprite_data.position[0],
            sprite_data.position[1],
            sprite_data.texture_path,
            sprite_data.texture_frame,
            sprite_data.texture_count
        );
        
        let sprite = sprite_cache.get(&key).unwrap();
        
        let uniforms = Uniforms {
            translation: [
                sprite_data.position[0],
                sprite_data.position[1],
                sprite_data.position[2],
                1.0,
            ],
            rotation: [0.0, 0.0, 0.0, 1.0],
            _padding: [0.0; 3],
        };
        queue.write_buffer(&sprite.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        render_pass.set_bind_group(0, &sprite.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &sprite.texture_bind_group, &[]);
        render_pass.set_bind_group(2, bind_group, &[]);  // используем переданный bind_group
        render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
        render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
    }
}