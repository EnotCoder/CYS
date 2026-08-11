use winit::window::Window;
use wgpu::*;
use wgpu::util::DeviceExt;
use crate::DepthBuffer;
use crate::api_components::pipeline::{create_render_pipeline, create_transparent_pipeline};

// ========================================================================
//  init: создание WgpuApp — вершины всего wgpu-пайплайна.
//
//  Настраивает instance -> surface -> adapter -> device/queue,
//  создаёт буферы, bind group'ы и два render pipeline
//  (обычный и прозрачный), затем конфигурирует surface.
// ========================================================================

// Uniform'ы-константы для WORLD-матрицы карты (передаются в шейдер).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Size {
    pub map_size: f32,
    pub aspect: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub night_factor: f32,
}

// Uniform'ы для UI-слоя (упрощённый набор: без позиции, один на кадр).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiUniforms {
    pub size: f32,
    pub aspect: f32,
    pub _padding: [f32; 2],
    pub night_factor: f32,
}

#[allow(dead_code)]
// Главная структура приложения: держит всё "железо" wgpu на протяжении жизни.
pub struct WgpuApp {
    // Базовые объекты wgpu, с которыми работает пайплайн.
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,

    // Связанное с динамическим storage-буфером спрайтов (group 0).
    pub dynamic_bind_group_layout: wgpu::BindGroupLayout,
    pub dynamic_uniform_buffer: wgpu::Buffer,
    pub dynamic_bind_group: wgpu::BindGroup,
    pub dynamic_alignment: u64,
    // Depth-состояния: обычный и прозрачный проходы.
    pub depth_stencil: wgpu::DepthStencilState,
    pub transparent_depth_stencil: wgpu::DepthStencilState,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_buffer: DepthBuffer,

    // Готовые пайплайны + конфигурация поверхности для смены размера.
    pub render_pipeline: wgpu::RenderPipeline,
    pub transparent_pipeline: wgpu::RenderPipeline,
    pub config: wgpu::SurfaceConfiguration,
    pub surface_caps: wgpu::SurfaceCapabilities,

    // Uniform'ы-константы карты (Size) с их bind group (group 2).
    pub size_buffer: wgpu::Buffer,
    pub size_bind_group: wgpu::BindGroup,

    // Uniform'ы UI (UiUniforms) с bind group.
    pub ui_uniform_buffer: wgpu::Buffer,
    pub ui_bind_group: wgpu::BindGroup,
}

impl WgpuApp {
    // Полный путь инициализации wgpu: вызывается один раз при старте приложения.
    pub fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            // PRIMARY — Vulkan (на Linux), без fallback на другие API.
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        let surface = instance.create_surface(window)
            .expect("Failed to create surface");

        // pollster::block_on — превращает async wgpu API в синхронный вызов.
        let adapter = pollster::block_on(Self::request_adapter(&instance, &surface));
        let (device, queue) = pollster::block_on(Self::request_device(&adapter));
        let surface_format = Self::pick_format(&surface, &adapter);
        let surface_caps = surface.get_capabilities(&adapter);

        let window_size = window.inner_size();

        let buffers = crate::init_buffers(window_size, &device);
        let shader_module = Self::load_shader(&device);

        // Uniform (Size) для карты — постоянный буфер, обновляется извне.
        let size_buffer = Self::create_size_buffer(&device);
        let size_bind_group_layout = Self::create_single_bind_group_layout(&device, "Size Bind Group Layout");
        let size_bind_group = Self::create_bind_group(&device, &size_bind_group_layout, &size_buffer, "Size Bind Group");

        // Uniform (UiUniforms) для UI-слоя.
        let ui_uniform_buffer = Self::create_ui_buffer(&device);
        let ui_bind_group_layout = Self::create_single_bind_group_layout(&device, "UI Bind Group Layout");
        let ui_bind_group = Self::create_bind_group(&device, &ui_bind_group_layout, &ui_uniform_buffer, "UI Bind Group");

        // Dynamic storage buffer — 1 write_buffer вместо N per-sprite
        // Подсчитываем, сколько спрайтов войдёт в лимит storage-буфера.
        let alignment = device.limits().min_storage_buffer_offset_alignment as u64;
        let max_binding = device.limits().max_storage_buffer_binding_size as u64;
        let max_sprites_by_limit = (max_binding / alignment) as usize;
        let max_dynamic = crate::constants::MAX_DYNAMIC_SPRITES.min(max_sprites_by_limit);
        let dynamic_buffer_size = max_dynamic as u64 * alignment;
        let dynamic_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Storage Buffer"),
            // STORAGE — читается шейдерами, COPY_DST — пишется из CPU.
            size: dynamic_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Binding size = alignment (один слот). С dynamic offset = i * alignment
        // шейдер читает sizeof(Uniforms) = 32 байта, что < alignment.
        let dynamic_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dynamic Bind Group"),
            layout: &buffers.dynamic_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                // Связываем весь буфер целиком, а конкретный спрайт
                // выбираем offset'ом при set_bind_group во время рендера.
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &dynamic_uniform_buffer,
                    offset: 0,
                    size: Some(wgpu::BufferSize::new(alignment).unwrap()),
                }),
            }],
        });

        // Layout пайплайна: порядок групп 0, 1, 2 должен совпадать
        // с set_bind_group(0/1/2, ...) во время рендера.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                Some(&buffers.dynamic_bind_group_layout),
                Some(&buffers.texture_bind_group_layout),
                Some(&size_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let render_pipeline = create_render_pipeline(
            &device, &pipeline_layout, &shader_module,
            surface_format, &buffers.depth_stencil,
        );
        let transparent_pipeline = create_transparent_pipeline(
            &device, &pipeline_layout, &shader_module,
            surface_format, &buffers.transparent_depth_stencil,
        );

        let config = surface_config(surface_format, window_size.width, window_size.height);
        surface.configure(&device, &config);

        Self {
            instance,
            device,
            queue,
            surface_format,
            dynamic_bind_group_layout: buffers.dynamic_bind_group_layout,
            dynamic_uniform_buffer,
            dynamic_bind_group,
            dynamic_alignment: alignment,
            depth_stencil: buffers.depth_stencil,
            transparent_depth_stencil: buffers.transparent_depth_stencil,
            texture_bind_group_layout: buffers.texture_bind_group_layout,
            depth_buffer: buffers.depth_buffer,
            render_pipeline,
            transparent_pipeline,
            config,
            surface_caps,
            size_buffer,
            size_bind_group,
            ui_uniform_buffer,
            ui_bind_group,
        }
    }

    // --- Приватные helper'ы ---

    // Находит совместимый с поверхностью GPU-адаптер (графическую карту).
    async fn request_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface<'_>) -> wgpu::Adapter {
        let adapter = instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(surface),
            ..Default::default()
        }).await.unwrap();
        println!("{}", adapter.get_info().name);
        adapter
    }

    // Запрашивает устройство (логический объект GPU) и очередь команд.
    async fn request_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // Стандартные лимиты: не требуем редких расширений.
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await
            .unwrap()
    }

    // Берём первый поддерживаемый surface-формат из списка возможностей.
    fn pick_format(surface: &wgpu::Surface, adapter: &wgpu::Adapter) -> wgpu::TextureFormat {
        surface.get_capabilities(adapter).formats[0]
    }

    // Компилирует шейдеры из файла shaders.wgsl (встраивается на этапе сборки).
    fn load_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        let code = include_str!("shaders.wgsl");
        device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(code.into()),
        })
    }

    // Общий layout для постоянного (non-dynamic) uniform-буфера:
    // один unbind binding без динамических offset.
    fn create_single_bind_group_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    // Связывает постоянный буфер с layout'ом в bind group (группа 2 для Size / UI).
    fn create_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buffer: &wgpu::Buffer, label: &str) -> wgpu::BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    // Создаёт буфер с дефолтными uniform'ами карты (заполняется позже из кода).
    fn create_size_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let size = Size { map_size: 1.0, aspect: 1.0, offset_x: 0.0, offset_y: 0.0, night_factor: 0.0 };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Size Buffer"),
            contents: bytemuck::bytes_of(&size),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }

    // Создаёт буфер дефолтных uniform'ов UI.
    fn create_ui_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let ui_uniforms = UiUniforms { size: 1.0, aspect: 1.0, _padding: [0.0; 2], night_factor: 0.0 };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI_Buffer"),
            contents: bytemuck::cast_slice(&[ui_uniforms]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }
}

// ========================================================================
//  Вспомогательные функции на уровне модуля
// ========================================================================

// Конфигурация поверхности (размер, формат, порядок кадров Fifo).
// Используется и на старте, и при изменении размера окна.
pub fn surface_config(format: wgpu::TextureFormat, width: u32, height: u32) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        // Fifo = вертикальная синхронизация (VSync), без разрывов кадров.
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Srgb,
        desired_maximum_frame_latency: crate::constants::DESIRED_FRAME_LATENCY,
    }
}
