use image::GenericImageView;

// ========================================================================
//  texture: загрузка и создание спрайтовых текстур (Rgba8UnormSrgb).
//
//  Одна текстура = wgpu::Texture + view (для семплирования в шейдере)
//  + sampler (настройки фильтрации). Всё создаётся из пикселей RGBA.
// ========================================================================

pub struct Texture {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    // Создаёт текстуру из сырых байтов PNG/JPEG (декодирование image-крейтом) 
    // и загружает пиксели в GPU через queue.write_texture.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let img = image::load_from_memory(bytes)?;
        // Конвертируем в RGBA8: wgpu потребует байты в этом формате.
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Srgb: цветовые значения конвертируются в линейное пространство.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TEXTURE_BINDING — семплируется в шейдере, COPY_DST — пишем с CPU.
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Копирование пикселей из CPU-памяти в GPU-текстуру.
        // bytes_per_row = 4 (RGBA) * ширину — обязательное выравнивание строк.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Пиксель-арт фильтруется Nearest (резкие квадраты, без размытия).
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        
        Ok(Self { _texture: texture, view, sampler })
    }
    
    // Удобный способ загрузить текстуру с диска по пути.
    // При любой ошибке (файл не найден/не декодируется) не паникуем,
    // а молча подставляем текстуру-заглушку (null.png).
    pub fn from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        label: &str,
    ) -> Self {
        match std::fs::read(path) {
            Ok(bytes) => {
                match Self::from_bytes(device, queue, &bytes, label) {
                    Ok(texture) => texture,
                    Err(e) => {
                        eprintln!("Failed to decode texture '{}': {}, loading null.png", path, e);
                        Self::load_null(device, queue, label)
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read texture '{}': {}, loading null.png", path, e);
                Self::load_null(device, queue, label)
            }
        }
    }

    // Создаёт текстуру из готовых пикселей RGBA (например, для курсора/UI),
    // в отличие от from_bytes не декодирует файлы — данные уже в памяти.
    // Использует Linear-фильтрацию, т.к. такие текстуры часто масштабируются.
    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self { _texture: texture, view, sampler }
    }
    
    // Текстура-заглушка (fallback) для случаев сбоя загрузки обычной текстуры.
    // Полный сбой здесь считается критической ошибкой — паникуем с сообщением.
    fn load_null(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        original_label: &str,
    ) -> Self {
        match std::fs::read(crate::constants::TEX_FALLBACK) {
            Ok(bytes) => {
                match Self::from_bytes(device, queue, &bytes, &format!("{}_fallback", original_label)) {
                    Ok(texture) => texture,
                    Err(e) => {
                        panic!("Critical error: {} exists but failed to decode: {}", crate::constants::TEX_FALLBACK, e);
                    }
                }
            }
            Err(e) => {
                panic!("Critical error: {} not found: {}", crate::constants::TEX_FALLBACK, e);
            }
        }
    }
}