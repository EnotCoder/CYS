// ========================================================================
//  api_components: низкоуровневые компоненты wgpu-пайплайна.
//
//  Модуль собирает воедино "железо" рендера:
//    buffers.rs — структуры данных (вершины, uniform'ы) и макеты биндингов;
//    init.rs    — инициализация WgpuApp: surface, device, буферы, пайплайны;
//    texture.rs — загрузка и создание текстур/сэмплеров;
//    render.rs  — отрисовка кадра по слоям (map / transparent / ui).
// ========================================================================

pub mod buffers;
pub mod init;
pub mod pipeline;
pub mod render;
pub mod texture;

// Продляем наружу всё содержимое подмодулей, чтобы работать было удобнее.
pub use buffers::*;
pub use init::*;
pub use render::*;
pub use texture::*;
