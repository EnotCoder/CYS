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
    
    // 1. Рендер карты (непрозрачные объекты, z=0)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Opaque Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.2,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(render_pipeline);
        
        for sprite_data in map_sprites {
            let key = format!(
                "map_{}_{}_{}_{:?}_{:?}",
                sprite_data.position[0],
                sprite_data.position[1],
                sprite_data.texture_path,
                sprite_data.texture_frame,
                sprite_data.texture_count
            );
            
            // СНАЧАЛА создаём спрайт, если нужно (мутабельный borrow)
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
        
        // ПОТОМ получаем и рисуем (только immutable borrow)
        for sprite_data in map_sprites {
            let key = format!(
                "map_{}_{}_{}_{:?}_{:?}",
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
            render_pass.set_bind_group(2, size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
            render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
        }
    }
    
    // 2. Рендер ковров (прозрачные, z=0.5)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Carpet Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(transparent_pipeline);
        
        // Сначала создаём все спрайты
        for sprite_data in carpet_sprites {
            let key = format!(
                "carpet_{}_{}_{}_{:?}_{:?}",
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
                sprite_cache.insert(key, new_sprite);
            }
        }
        
        // Потом рисуем
        for sprite_data in carpet_sprites {
            let key = format!(
                "carpet_{}_{}_{}_{:?}_{:?}",
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
            render_pass.set_bind_group(2, size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
            render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
        }
    }
    
    // 3. Рендер декора (прозрачные, z=1.0)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Decor Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(transparent_pipeline);
        
        // Сначала создаём все спрайты
        for sprite_data in decor_sprites {
            let key = format!(
                "decor_{}_{}_{}_{:?}_{:?}",
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
                sprite_cache.insert(key, new_sprite);
            }
        }
        
        // Потом рисуем
        for sprite_data in decor_sprites {
            let key = format!(
                "decor_{}_{}_{}_{:?}_{:?}",
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
            render_pass.set_bind_group(2, size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
            render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
        }
    }
    
    // 4. Рендер курсора (z=2.0)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cursor Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(transparent_pipeline);
        
        // Сначала создаём все спрайты
        for sprite_data in cursor_sprites {
            let key = format!(
                "cursor_{}_{}_{}_{:?}_{:?}",
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
                sprite_cache.insert(key, new_sprite);
            }
        }
        
        // Потом рисуем
        for sprite_data in cursor_sprites {
            let key = format!(
                "cursor_{}_{}_{}_{:?}_{:?}",
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
            render_pass.set_bind_group(2, size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
            render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
        }
    }
    
    // 5. Рендер UI (z=3.0)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(transparent_pipeline);
        
        // Сначала создаём все спрайты
        for sprite_data in ui_sprites {
            let key = format!(
                "ui_{}_{}_{}_{:?}_{:?}",
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
                sprite_cache.insert(key, new_sprite);
            }
        }
        
        // Потом рисуем
        for sprite_data in ui_sprites {
            let key = format!(
                "ui_{}_{}_{}_{:?}_{:?}",
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
            render_pass.set_bind_group(2, ui_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_index_buffer(sprite.index_buffer.slice(..), sprite.index_format);
            render_pass.draw_indexed(0..sprite.index_count, 0, 0..1);
        }
    }
    
    queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}