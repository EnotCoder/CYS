use winit::dpi::PhysicalSize;

// ========================================================================
//  buffers: данные, которые попадают в GPU-буферы, и макеты биндингов.
//
//  Здесь живут glsl/wgsl-совместимые структуры (Vertex, Uniforms),
//  буфер глубины и 3 макета bind group layout, общих для всех пайплайнов.
// ========================================================================

// Вершина спрайта: позиция (x,y,w) и текстурные координаты (u,v).
// Шаблон должен повторять структуру WBGL-шейдеров (см. vertex_buffer_layout).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

// Uniform на спрайт: позиция/альфа-канал и поворот (2 x vec4).
// Совпадает со структурой Uniforms в шейдерах WGSL.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub translation: [f32; 4],
    pub rotation: [f32; 4],
}

// Буфер глубины: текстура Depth32Float (равная размеру окна) + её view.
// `_texture` хранится только чтобы текстура не унечтожалась, рисуем мы
// через `view`. Пересоздаётся целиком при изменении размера окна.
pub struct DepthBuffer {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}



impl DepthBuffer {
    // Создаёт текстуру-прикрепление глубины размером точно под текущее окно.
    pub fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Depth32Float подходит и для depth_stencil, и для чтения-теста.
            format: wgpu::TextureFormat::Depth32Float,
            // Используется только как render attachment — в шейдеры не биндится.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        Self { _texture: texture, view }
    }
    
    // При изменении размера окна старая текстура не подходит —
    // целиком пересоздаём новую и подменяем ссылки.
    pub fn resize(&mut self, device: &wgpu::Device, new_size: PhysicalSize<u32>) {
        let new = Self::new(device, new_size);
        self._texture = new._texture;
        self.view = new.view;
    }
}

// Набор "общего" состояния, которое создаётся один раз и используется
// сразу всеми пайплайнами (обычным и прозрачным), а также при рендере.
pub struct Buffers{
    // Layout для dynamic storage buffer (group 0): один слот на спрайт.
    pub dynamic_bind_group_layout: wgpu::BindGroupLayout,
    // Текстура-прикрепление глубины на весь кадр.
    pub depth_buffer: DepthBuffer,
    // Тест глубины для непрозрачных объектов (обычный path).
    pub depth_stencil: wgpu::DepthStencilState,
    // Пустой тест глубины для прозрачных слоёв (транспарентный path).
    pub transparent_depth_stencil: wgpu::DepthStencilState,
    // Layout для пары "текстура + сэмплер" (группа 1), общий для спрайтов.
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

// Собирает все общие bind group layout и depth-стешиль-состояния в одну
// структуру-Buffers. Нужно вызывать ОДИН раз при старте до создания пайплайнов.
pub fn init_buffers(
    window_size: PhysicalSize<u32>,
    device: &wgpu::Device,
) -> Buffers{
    // Layout для dynamic storage buffer (group 0).
    // Storage вместо Uniform из-за лимита GPU-устройства:
    // max_uniform_buffer_binding_size = 65536 (AMD RADV).
    let dynamic_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Dynamic Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                // uniform'ы спрайтов читаются в шейдерах обеих стадий.
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    // Storage buffer: dynamic offset даёт "срез" на конкретный спрайт.
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    // Гарантируем, что в слоте уместится минимум один Uniforms.
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Uniforms>() as u64),
                },
                count: None,
            },
        ],
    });

    // Создаём depth texture
    let depth_buffer = DepthBuffer::new(&device, window_size);

    // Настройка depth_stencil для render pipeline
    let depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        // Пишем глубину и рисуем только поверх уже отрисованного.
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };

    // Для прозрачных объектов тест глубины отключён (Always) —
    // порядок определяется очередностью спрайтов в буфере.
    // Запись глубины всё равно включена, чтобы не затирать уже нарисованное.
    let transparent_depth_stencil = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };

    // Layout для пары "текстура + сэмплер" (group 1) — спрайтовые текстуры.
    let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                // Текстура нужна только во фрагментном шейдере (семплирование).
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    // Float { filterable } — фильтрация маг/мин (Nearest/Linear).
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                // Сэмплер сглаживания, используется фрагментным шейдером.
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    // Собираем всё в одну структуру для передачи в пайплайны и рендер.
    Buffers {
        dynamic_bind_group_layout,
        transparent_depth_stencil,
        depth_buffer,
        depth_stencil,
        texture_bind_group_layout,
    }
}