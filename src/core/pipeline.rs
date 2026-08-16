// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  pipeline.rs — создание render-пайплайнов и layout вершин
// ========================================================================
//  Содержит построение обычного и прозрачного pipeline'ов wgpu,
//  а также описание расположения атрибутов вершин (VertexBufferLayout).
// ========================================================================

use wgpu::*;
use crate::Vertex;

// Layout атрибутов вершин: position = location 0 (vec3),
// tex_coord = location 1 (vec2). Должен совпадать с buffers::Vertex.
pub fn vertex_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        // Шаг между вершинами = размер всей структуры Vertex.
        array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            // Атрибут tex_coord начинается сразу после position.
            VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                shader_location: 1,
                format: VertexFormat::Float32x2,
            },
        ],
    }
}

// Основной пайплайн: обычное смешивание и реальный тест глубины.
pub fn create_render_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    format: TextureFormat,
    depth_stencil: &DepthStencilState,
) -> RenderPipeline {
    let comp_opts = PipelineCompilationOptions::default();
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: VertexState {
            buffers: &[Some(vertex_buffer_layout())],
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: comp_opts.clone(),
        },
        fragment: Some(FragmentState {
            targets: &[Some(ColorTargetState {
                format,
                // Стандартное alpha-смешивание: src * srcAlpha + dst * (1 - srcAlpha).
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                }),
                write_mask: ColorWrites::ALL,
            })],
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: comp_opts.clone(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            // Способ разворота вершин: CCW = против часовой стрелки.
            front_face: FrontFace::Ccw,
            // Без отсечения задних граней — спрайты считаются двусторонними.
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(depth_stencil.clone()),
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

// Прозрачный пайплайн: отличается только типом смешивания
// (ALPHA_BLENDING) и отключённым тестом глубины (CompareFunction::Always).
pub fn create_transparent_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    format: TextureFormat,
    depth_stencil: &DepthStencilState,
) -> RenderPipeline {
    let comp_opts = PipelineCompilationOptions::default();
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Transparent Pipeline"),
        layout: Some(layout),
        vertex: VertexState {
            buffers: &[Some(vertex_buffer_layout())],
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: comp_opts.clone(),
        },
        fragment: Some(FragmentState {
            targets: &[Some(ColorTargetState {
                format,
                // Встроенное альфа-смешивание из коробки wgpu.
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: comp_opts.clone(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(depth_stencil.clone()),
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
