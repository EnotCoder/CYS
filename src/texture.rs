// texture.rs
use wgpu::{util::DeviceExt, *};
use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,      // Сама текстура на GPU
    pub view: wgpu::TextureView,     // Представление текстуры
    pub sampler: wgpu::Sampler,      // Сэмплер (как читать текстуру)
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Загружаем изображение из байтов с помощью библиотеки image
        let img = image::load_from_memory(bytes)?;
        // 2. Конвертируем в формат RGBA8 (32 бита на пиксель: R,G,B,A)
        let rgba = img.to_rgba8();
        // 3. Получаем размеры изображения
        let dimensions = img.dimensions();
        
        // 4. Создаём размер текстуры
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,  // 2D текстура, не массив
        };
        
        // 5. Создаём текстуру на GPU
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,              // Без MIP-уровней (пока)
            sample_count: 1,                 // Без мультисэмплинга
            dimension: wgpu::TextureDimension::D2,  // 2D текстура
            format: wgpu::TextureFormat::Rgba8UnormSrgb,  // Формат RGBA8
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // 6. Записываем данные пикселей в текстуру
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,  // Данные пикселей
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),  // 4 байта на пиксель * ширина
                rows_per_image: Some(dimensions.1),
            },
            size,
        );
        
        // 7. Создаём представление текстуры (способ доступа)
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // 8. Создаём сэмплер (определяет как текстура накладывается)
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,   // Повторять по U
            address_mode_v: wgpu::AddressMode::Repeat,   // Повторять по V
            address_mode_w: wgpu::AddressMode::Repeat,   // Повторять по W
            mag_filter: wgpu::FilterMode::Linear,        // Увеличение: линейное
            min_filter: wgpu::FilterMode::Nearest,       // Уменьшение: ближайший
            mipmap_filter: wgpu::FilterMode::Nearest,    // MIP-фильтр
            ..Default::default()
        });
        
        Ok(Self { texture, view, sampler })
    }
    
    pub fn from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        label: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(device, queue, &bytes, label)
    }
}