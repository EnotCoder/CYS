// Структура uniform (только трансляция)
struct Uniforms {
    translation: vec4<f32>,
    rotation: vec4<f32>,
};

struct Size {
    map_size: f32,
};

struct UiUniforms {
    scale: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Текстура и сэмплер
@group(1) @binding(0)
var my_texture: texture_2d<f32>;

@group(1) @binding(1)
var my_sampler: sampler;

@group(2) @binding(0)
var<uniform> size_uniform: Size;

@group(2) @binding(0)
var<uniform> ui_uniforms: UiUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

// Вращение вокруг X
fn rotate_x(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x,
        vertex.y * c - vertex.z * s,
        vertex.y * s + vertex.z * c
    );
}

// Вращение вокруг Y
fn rotate_y(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x * c + vertex.z * s,
        vertex.y,
        -vertex.x * s + vertex.z * c
    );
}

// Вращение вокруг Z
fn rotate_z(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x * c - vertex.y * s,
        vertex.x * s + vertex.y * c,
        vertex.z
    );
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    
    // Применяем вращение
    var rotated = position;
    rotated = rotate_x(rotated, uniforms.rotation.x);
    rotated = rotate_y(rotated, uniforms.rotation.y);
    rotated = rotate_z(rotated, uniforms.rotation.z);
    
    // Применяем трансляцию
    let world_pos = rotated + uniforms.translation.xyz;
    
    let size = 0.223 * size_uniform.map_size;

    // Без матрицы! Просто передаём координаты
    output.position = vec4<f32>(world_pos.x * size, world_pos.y * size, 0.0, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

@fragment
fn fs_main(
    @location(0) tex_coord: vec2<f32>,
) -> @location(0) vec4<f32> {
    return textureSample(my_texture, my_sampler, tex_coord);
}