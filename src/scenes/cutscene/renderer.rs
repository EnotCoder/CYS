// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  renderer — GPU-часть вступительной кат-сцены.
//
//  Два offscreen-таргета: текущий и следующий план. Каждый план рисуется
//  своим fullscreen/инстансированным пайплайном, затем композитор делает
//  либо спокойный blit, либо быстрый рывок с радиальным смазом.
// ========================================================================

use super::stars::{build_instances, generate_stars, Star, StarInstance};
use super::timeline::{jump_params, next_plane_of, plane_of, plane_transform, Plane, Stage};
use crate::core::{DepthBuffer, Texture};
use bytemuck::{Pod, Zeroable};
use image::GenericImageView;

const STAR_MODULE: &str = include_str!("shaders/stars.wgsl");
const PLANET_MODULE: &str = include_str!("shaders/planets.wgsl");
const COMPOSITE_MODULE: &str = include_str!("shaders/composite.wgsl");
const HINT_MODULE: &str = include_str!("shaders/hint.wgsl");

const REQUIRED_ASSETS: [&str; 5] = [
    "assets/tex/cat_scene/sun.png",
    "assets/tex/cat_scene/earth.png",
    "assets/tex/cat_scene/moon.png",
    "assets/tex/cat_scene/continent.png",
    "assets/tex/cat_scene/sea.png",
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StarUniforms {
    target_size: [f32; 2],
    time: f32,
    visibility: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PlanetUniforms {
    resolution: [f32; 2],
    time: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompositeUniforms {
    a: [f32; 4],
    b: [f32; 4],
    misc: [f32; 4],
    focus: [f32; 4],
    extra: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HintUniforms {
    rect: [f32; 4],
    style: [f32; 4],
}

struct Target {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(
        device: &wgpu::Device,
        label: &str,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }
}

fn uniform_buffer(device: &wgpu::Device, size: u64, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn uniform_layout(
    device: &wgpu::Device,
    visibility: wgpu::ShaderStages,
    label: &str,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn uniform_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        }],
    })
}

fn texture_layout(
    device: &wgpu::Device,
    textures: u32,
    sampler_binding: u32,
    sampler_type: wgpu::SamplerBindingType,
    label: &str,
) -> wgpu::BindGroupLayout {
    let mut entries = Vec::with_capacity(textures as usize + 1);
    for binding in 0..textures {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        });
    }
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: sampler_binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(sampler_type),
        count: None,
    });
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &entries,
    })
}

fn texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    views: &[&wgpu::TextureView],
    sampler: &wgpu::Sampler,
    sampler_binding: u32,
    label: &str,
) -> wgpu::BindGroup {
    let mut entries: Vec<wgpu::BindGroupEntry> = views
        .iter()
        .enumerate()
        .map(|(i, view)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: wgpu::BindingResource::TextureView(view),
        })
        .collect();
    entries.push(wgpu::BindGroupEntry {
        binding: sampler_binding,
        resource: wgpu::BindingResource::Sampler(sampler),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &entries,
    })
}

fn shader(device: &wgpu::Device, source: &str, label: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn fullscreen_pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    entry: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_fullscreen"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn load_texture(device: &wgpu::Device, queue: &wgpu::Queue, path: &str, label: &str) -> Texture {
    let bytes = crate::core::asset::load_bytes(path)
        .unwrap_or_else(|error| panic!("cutscene asset missing: {path} ({error})"));
    let image = image::load_from_memory(&bytes)
        .unwrap_or_else(|error| panic!("cutscene asset cannot be decoded: {path} ({error})"));
    let (w, h) = image.dimensions();
    Texture::from_rgba(
        device,
        queue,
        &image.to_rgba8(),
        w,
        h,
        label,
        wgpu::FilterMode::Nearest,
    )
}

pub struct CutsceneRenderer {
    size: (u32, u32),
    target_format: wgpu::TextureFormat,
    stars: Vec<Star>,
    instances: Vec<StarInstance>,

    star_uniform: wgpu::Buffer,
    star_instances: wgpu::Buffer,
    star_pipeline: wgpu::RenderPipeline,
    star_uniform_bg: wgpu::BindGroup,

    planet_uniform: wgpu::Buffer,
    planet_uniform_bg: wgpu::BindGroup,
    solar_pipeline: wgpu::RenderPipeline,
    continent_pipeline: wgpu::RenderPipeline,
    planet_bg: wgpu::BindGroup,

    _sun_tex: Texture,
    _earth_tex: Texture,
    _moon_tex: Texture,
    _continent_tex: Texture,
    _sea_tex: Texture,
    sampler: wgpu::Sampler,

    composite_uniform: wgpu::Buffer,
    composite_uniform_bg: wgpu::BindGroup,
    pair_layout: wgpu::BindGroupLayout,
    blit_pipeline: wgpu::RenderPipeline,
    transition_pipeline: wgpu::RenderPipeline,
    plane_a_bg: Option<wgpu::BindGroup>,
    plane_b_bg: Option<wgpu::BindGroup>,
    shop_bg: Option<wgpu::BindGroup>,

    hint_uniform: wgpu::Buffer,
    hint_uniform_bg: wgpu::BindGroup,
    hint_pipeline: wgpu::RenderPipeline,
    hint_tex: Option<Texture>,
    hint_bg: Option<wgpu::BindGroup>,
    hint_size: (u32, u32),

    plane_a: Option<Target>,
    plane_b: Option<Target>,
    shop: Option<Target>,
    snapshot_depth: Option<DepthBuffer>,
    shop_captured: bool,
}

impl CutsceneRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let sun_tex = load_texture(device, queue, REQUIRED_ASSETS[0], "cutscene sun");
        let earth_tex = load_texture(device, queue, REQUIRED_ASSETS[1], "cutscene earth");
        let moon_tex = load_texture(device, queue, REQUIRED_ASSETS[2], "cutscene moon");
        let continent_tex = load_texture(device, queue, REQUIRED_ASSETS[3], "cutscene continent");
        let sea_tex = load_texture(device, queue, REQUIRED_ASSETS[4], "cutscene sea");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let pixel_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let star_uniform = uniform_buffer(device, 16, "Cutscene Star Uniforms");
        let star_uniform_layout = uniform_layout(
            device,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            "Cutscene Star Uniform Layout",
        );
        let star_uniform_bg = uniform_bind_group(
            device,
            &star_uniform_layout,
            &star_uniform,
            "Cutscene Star Uniform BG",
        );
        let stars = generate_stars();
        let instances = vec![StarInstance::zeroed(); stars.len()];
        let star_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cutscene Star Instances"),
            size: (instances.len() * std::mem::size_of::<StarInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let star_module = shader(device, STAR_MODULE, "Cutscene Stars WGSL");
        let star_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cutscene Star Layout"),
            bind_group_layouts: &[Some(&star_uniform_layout)],
            immediate_size: 0,
        });
        let star_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cutscene Star Pipeline"),
            layout: Some(&star_layout),
            vertex: wgpu::VertexState {
                module: &star_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<StarInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &star_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let planet_uniform = uniform_buffer(device, 16, "Cutscene Planet Uniforms");
        let planet_uniform_layout = uniform_layout(
            device,
            wgpu::ShaderStages::FRAGMENT,
            "Cutscene Planet Uniform Layout",
        );
        let planet_uniform_bg = uniform_bind_group(
            device,
            &planet_uniform_layout,
            &planet_uniform,
            "Cutscene Planet Uniform BG",
        );
        let planet_pair_layout = texture_layout(
            device,
            5,
            5,
            wgpu::SamplerBindingType::NonFiltering,
            "Cutscene Planet Texture Layout",
        );
        let planet_bg = texture_bind_group(
            device,
            &planet_pair_layout,
            &[
                &sun_tex.view,
                &earth_tex.view,
                &moon_tex.view,
                &continent_tex.view,
                &sea_tex.view,
            ],
            &pixel_sampler,
            5,
            "Cutscene Planet Textures",
        );
        let planet_module = shader(device, PLANET_MODULE, "Cutscene Planets WGSL");
        let planet_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cutscene Planet Layout"),
            bind_group_layouts: &[Some(&planet_uniform_layout), Some(&planet_pair_layout)],
            immediate_size: 0,
        });
        let over_stars = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let solar_pipeline = fullscreen_pipeline(
            device,
            &planet_module,
            &planet_layout,
            "fs_solar",
            format,
            Some(over_stars),
            "Cutscene Solar Pipeline",
        );
        let continent_pipeline = fullscreen_pipeline(
            device,
            &planet_module,
            &planet_layout,
            "fs_continent",
            format,
            None,
            "Cutscene Continent Pipeline",
        );

        let composite_uniform = uniform_buffer(device, 80, "Cutscene Composite Uniforms");
        let composite_uniform_layout = uniform_layout(
            device,
            wgpu::ShaderStages::FRAGMENT,
            "Cutscene Composite Uniform Layout",
        );
        let composite_uniform_bg = uniform_bind_group(
            device,
            &composite_uniform_layout,
            &composite_uniform,
            "Cutscene Composite Uniform BG",
        );
        let pair_layout = texture_layout(
            device,
            1,
            1,
            wgpu::SamplerBindingType::Filtering,
            "Cutscene Composite Pair Layout",
        );
        let composite_module = shader(device, COMPOSITE_MODULE, "Cutscene Composite WGSL");
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cutscene Composite Layout"),
            bind_group_layouts: &[
                Some(&composite_uniform_layout),
                Some(&pair_layout),
                Some(&pair_layout),
            ],
            immediate_size: 0,
        });
        let blit_pipeline = fullscreen_pipeline(
            device,
            &composite_module,
            &composite_layout,
            "fs_blit",
            format,
            None,
            "Cutscene Blit Pipeline",
        );
        let transition_pipeline = fullscreen_pipeline(
            device,
            &composite_module,
            &composite_layout,
            "fs_transition",
            format,
            None,
            "Cutscene Transition Pipeline",
        );

        let hint_uniform = uniform_buffer(device, 32, "Cutscene Hint Uniforms");
        let hint_uniform_layout = uniform_layout(
            device,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            "Cutscene Hint Uniform Layout",
        );
        let hint_uniform_bg = uniform_bind_group(
            device,
            &hint_uniform_layout,
            &hint_uniform,
            "Cutscene Hint Uniform BG",
        );
        let hint_module = shader(device, HINT_MODULE, "Cutscene Hint WGSL");
        let hint_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cutscene Hint Layout"),
            bind_group_layouts: &[Some(&hint_uniform_layout), Some(&pair_layout)],
            immediate_size: 0,
        });
        let hint_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cutscene Hint Pipeline"),
            layout: Some(&hint_layout),
            vertex: wgpu::VertexState {
                module: &hint_module,
                entry_point: Some("vs_fullscreen"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &hint_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let mut this = Self {
            size: (0, 0),
            target_format: format,
            stars,
            instances,
            star_uniform,
            star_instances,
            star_pipeline,
            star_uniform_bg,
            planet_uniform,
            planet_uniform_bg,
            solar_pipeline,
            continent_pipeline,
            planet_bg,
            _sun_tex: sun_tex,
            _earth_tex: earth_tex,
            _moon_tex: moon_tex,
            _continent_tex: continent_tex,
            _sea_tex: sea_tex,
            sampler,
            composite_uniform,
            composite_uniform_bg,
            pair_layout,
            blit_pipeline,
            transition_pipeline,
            plane_a_bg: None,
            plane_b_bg: None,
            shop_bg: None,
            hint_uniform,
            hint_uniform_bg,
            hint_pipeline,
            hint_tex: None,
            hint_bg: None,
            hint_size: (1, 1),
            plane_a: None,
            plane_b: None,
            shop: None,
            snapshot_depth: None,
            shop_captured: false,
        };
        this.resize(device, size);
        this
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if size == self.size && self.plane_a.is_some() {
            return;
        }
        self.size = size;
        self.plane_a = Some(Target::new(
            device,
            "Cutscene Plane A",
            size,
            self.target_format,
        ));
        self.plane_b = Some(Target::new(
            device,
            "Cutscene Plane B",
            size,
            self.target_format,
        ));
        self.shop = Some(Target::new(
            device,
            "Cutscene Shop Snapshot",
            size,
            self.target_format,
        ));
        self.snapshot_depth = Some(DepthBuffer::new(
            device,
            winit::dpi::PhysicalSize {
                width: size.0.max(1),
                height: size.1.max(1),
            },
        ));
        let a = &self.plane_a.as_ref().unwrap().view;
        let b = &self.plane_b.as_ref().unwrap().view;
        let s = &self.shop.as_ref().unwrap().view;
        self.plane_a_bg = Some(texture_bind_group(
            device,
            &self.pair_layout,
            &[a],
            &self.sampler,
            1,
            "Cutscene Plane A BG",
        ));
        self.plane_b_bg = Some(texture_bind_group(
            device,
            &self.pair_layout,
            &[b],
            &self.sampler,
            1,
            "Cutscene Plane B BG",
        ));
        self.shop_bg = Some(texture_bind_group(
            device,
            &self.pair_layout,
            &[s],
            &self.sampler,
            1,
            "Cutscene Shop BG",
        ));
        self.shop_captured = false;
    }

    pub fn shop_captured(&self) -> bool {
        self.shop_captured
    }

    pub fn reset_shop(&mut self) {
        self.shop_captured = false;
    }

    /// Маленькая подсказка о пропуске; текст растеризуется один раз до старта.
    pub fn set_hint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    ) {
        self.hint_tex = Some(Texture::from_rgba(
            device,
            queue,
            &pixels,
            width,
            height,
            "Cutscene Skip Hint",
            wgpu::FilterMode::Linear,
        ));
        self.hint_bg = Some(texture_bind_group(
            device,
            &self.pair_layout,
            &[&self.hint_tex.as_ref().unwrap().view],
            &self.sampler,
            1,
            "Cutscene Skip Hint BG",
        ));
        self.hint_size = (width.max(1), height.max(1));
    }

    /// Снимок текущего фона меню (карта магазина + декор) в отдельную текстуру.
    pub fn capture_shop(
        &mut self,
        ecs: &mut crate::EcsAdapter,
        wgpu_app: &crate::core::WgpuApp,
        window_size: (u32, u32),
    ) {
        let aspect = window_size.0 as f32 / window_size.1.max(1) as f32;
        let map_size = crate::core::util::ui_fit_scale(
            aspect,
            crate::core::constants::MENU_UI_HALF_W,
            crate::core::constants::MENU_UI_HALF_H,
        );
        let size_data = crate::core::Size {
            map_size,
            aspect,
            offset_x: 0.0,
            offset_y: 0.0,
            night_factor: 0.0,
            light_count: 0,
            _padding: [0.0; 2],
        };
        let ui_data = crate::core::UiUniforms {
            size: map_size,
            aspect,
            _padding1: [0.0; 2],
            night_factor: 0.0,
            light_count: 0,
            _padding2: [0.0; 2],
        };
        let queue = &wgpu_app.queue;
        queue.write_buffer(&wgpu_app.size_buffer, 0, bytemuck::cast_slice(&[size_data]));
        queue.write_buffer(
            &wgpu_app.ui_uniform_buffer,
            0,
            bytemuck::cast_slice(&[ui_data]),
        );

        let (map, carpet, light, decor, npc, weather, cursor, _ui) = ecs.get_sprites_by_layer(None);
        let empty: Vec<crate::SpriteRenderData> = Vec::new();
        let shop_view = &self.shop.as_ref().unwrap().view;
        let depth_view = &self.snapshot_depth.as_ref().unwrap().view;
        crate::core::render::render_to_view(
            shop_view,
            &wgpu_app.device,
            queue,
            &wgpu_app.render_pipeline,
            &wgpu_app.transparent_pipeline,
            depth_view,
            &map,
            &carpet,
            &light,
            &decor,
            &npc,
            &weather,
            &cursor,
            &empty,
            &wgpu_app.size_bind_group,
            &wgpu_app.ui_bind_group,
            &mut ecs.sprite_cache,
            &mut ecs.texture_cache,
            &wgpu_app.dynamic_uniform_buffer,
            &wgpu_app.dynamic_bind_group,
            wgpu_app.dynamic_alignment,
            &[],
            &wgpu_app.light_buffer,
            None,
        );
        self.shop_captured = true;
    }

    /// Обновляет CPU-данные звёзд и uniform'ы на кадр.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        viewport: (f32, f32),
        time: f32,
        visibility: f32,
        camera: f32,
        wrap: bool,
    ) {
        self.instances = build_instances(&self.stars, viewport, time, visibility, camera, wrap);
        queue.write_buffer(
            &self.star_instances,
            0,
            bytemuck::cast_slice(&self.instances),
        );
        queue.write_buffer(
            &self.star_uniform,
            0,
            bytemuck::cast_slice(&[StarUniforms {
                target_size: [viewport.0, viewport.1],
                time,
                visibility,
            }]),
        );
        queue.write_buffer(
            &self.planet_uniform,
            0,
            bytemuck::cast_slice(&[PlanetUniforms {
                resolution: [viewport.0, viewport.1],
                time,
                _pad: 0.0,
            }]),
        );
    }

    fn render_plane(
        &self,
        plane: Plane,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cutscene Plane Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        match plane {
            Plane::Stars => {
                pass.set_pipeline(&self.star_pipeline);
                pass.set_bind_group(0, &self.star_uniform_bg, &[]);
                pass.set_vertex_buffer(0, self.star_instances.slice(..));
                pass.draw(0..6, 0..self.stars.len() as u32);
            }
            Plane::Solar => {
                // Те же 296 звёзд, что и в отдельной звёздной сцене.
                pass.set_pipeline(&self.star_pipeline);
                pass.set_bind_group(0, &self.star_uniform_bg, &[]);
                pass.set_vertex_buffer(0, self.star_instances.slice(..));
                pass.draw(0..6, 0..self.stars.len() as u32);

                pass.set_pipeline(&self.solar_pipeline);
                pass.set_bind_group(0, &self.planet_uniform_bg, &[]);
                pass.set_bind_group(1, &self.planet_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            Plane::Continent => {
                pass.set_pipeline(&self.continent_pipeline);
                pass.set_bind_group(0, &self.planet_uniform_bg, &[]);
                pass.set_bind_group(1, &self.planet_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            Plane::Shop | Plane::Dark => {}
        }
    }

    fn composite_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        transition: bool,
        a_bg: &wgpu::BindGroup,
        b_bg: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cutscene Composite Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(if transition {
            &self.transition_pipeline
        } else {
            &self.blit_pipeline
        });
        pass.set_bind_group(0, &self.composite_uniform_bg, &[]);
        pass.set_bind_group(1, a_bg, &[]);
        pass.set_bind_group(2, b_bg, &[]);
        pass.draw(0..3, 0..1);
    }

    fn hint_pass(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        time: f32,
    ) {
        let Some(bg) = self.hint_bg.as_ref() else {
            return;
        };
        let viewport = (self.size.0 as f32, self.size.1 as f32);
        let width = (self.hint_size.0 as f32 * 0.9).min(viewport.0 * 0.38);
        let height = width * self.hint_size.1 as f32 / self.hint_size.0 as f32;
        let x = (viewport.0 - width) * 0.5;
        let y = viewport.1 - height - 18.0;
        let uniform = HintUniforms {
            rect: [
                x / viewport.0,
                y / viewport.1,
                width / viewport.0,
                height / viewport.1,
            ],
            style: [0.26, time, 0.0, 0.0],
        };
        queue.write_buffer(&self.hint_uniform, 0, bytemuck::bytes_of(&uniform));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cutscene Hint Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.hint_pipeline);
        pass.set_bind_group(0, &self.hint_uniform_bg, &[]);
        pass.set_bind_group(1, bg, &[]);
        pass.draw(0..6, 0..1);
    }

    /// Рисует один кадр заставки. Возвращает false, если surface недоступен.
    pub fn render_frame(
        &mut self,
        surface: &wgpu::Surface,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stage: Stage,
        progress: f32,
        time: f32,
        fade: f32,
        show_hint: bool,
    ) -> bool {
        if self.size.0 == 0 || self.size.1 == 0 {
            return false;
        }
        let current = surface.get_current_texture();
        let frame = match current {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return false,
        };
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Cutscene Encoder"),
        });
        let aspect = self.size.0 as f32 / self.size.1 as f32;

        if stage == Stage::Dark {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Cutscene Dark Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        } else {
            let a_view = &self.plane_a.as_ref().unwrap().view;
            let b_view = &self.plane_b.as_ref().unwrap().view;
            let a_bg = self.plane_a_bg.as_ref().unwrap();
            let b_bg = self.plane_b_bg.as_ref().unwrap();
            let shop_bg = self.shop_bg.as_ref().unwrap();

            if let Some(next) = next_plane_of(stage) {
                let current_plane = plane_of(stage);
                self.render_plane(current_plane, a_view, &mut encoder);
                let b_bind = if next == Plane::Shop && self.shop_captured {
                    shop_bg
                } else {
                    self.render_plane(next, b_view, &mut encoder);
                    b_bg
                };
                let transform = plane_transform(current_plane);
                let arrived = plane_transform(next);
                let jump = jump_params(stage);
                let uniform = CompositeUniforms {
                    a: [
                        transform.center[0],
                        transform.center[1],
                        transform.zoom,
                        jump.a_end_zoom,
                    ],
                    b: [arrived.center[0], arrived.center[1], arrived.zoom, progress],
                    misc: [aspect, jump.blur, 1.0, time],
                    focus: [
                        jump.focus[0],
                        jump.focus[1],
                        jump.b_start_center[0],
                        jump.b_start_center[1],
                    ],
                    extra: [jump.b_start_zoom, 0.0, 0.0, 0.0],
                };
                queue.write_buffer(&self.composite_uniform, 0, bytemuck::bytes_of(&uniform));
                self.composite_pass(&mut encoder, &surface_view, true, a_bg, b_bind);
            } else {
                let plane = plane_of(stage);
                let source_bg = if plane == Plane::Shop && self.shop_captured {
                    shop_bg
                } else {
                    self.render_plane(plane, a_view, &mut encoder);
                    a_bg
                };
                let transform = plane_transform(plane);
                let uniform = CompositeUniforms {
                    a: [
                        transform.center[0],
                        transform.center[1],
                        transform.zoom,
                        transform.zoom,
                    ],
                    b: [0.5, 0.5, 1.0, 0.0],
                    misc: [aspect, 0.0, fade, time],
                    focus: [0.5, 0.5, 0.5, 0.5],
                    extra: [1.0, 0.0, 0.0, 0.0],
                };
                queue.write_buffer(&self.composite_uniform, 0, bytemuck::bytes_of(&uniform));
                self.composite_pass(&mut encoder, &surface_view, false, source_bg, source_bg);
            }
        }

        if show_hint {
            self.hint_pass(queue, &mut encoder, &surface_view, time);
        }

        queue.submit(std::iter::once(encoder.finish()));
        queue.present(frame);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::REQUIRED_ASSETS;
    use std::path::Path;

    #[test]
    fn required_cutscene_assets_exist() {
        for path in REQUIRED_ASSETS {
            assert!(Path::new(path).exists(), "missing cutscene asset: {path}");
        }
    }
}
