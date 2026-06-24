// Структура uniform (только трансляция)
struct Uniforms {
    translation: vec4<f32>,
    rotation: vec4<f32>,
};

struct Size {
    map_size: f32,
    aspect: f32,
    offset_x: f32,
    offset_y: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Текстура и сэмплер
@group(1) @binding(0)
var my_texture: texture_2d<f32>;

@group(1) @binding(1)
var my_sampler: sampler;

@group(2) @binding(0)
var<uniform> size_uniform: Size;

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
    let cam_x = (world_pos.x - size_uniform.offset_x) * size / size_uniform.aspect;
    let cam_y = (world_pos.y - size_uniform.offset_y) * size;

    // Без матрицы! Просто передаём координаты
    output.position = vec4<f32>(cam_x, cam_y, 0.0, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

@fragment
fn fs_main(
    @location(0) tex_coord: vec2<f32>,
) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, tex_coord);
    return vec4<f32>(color.rgb, color.a * uniforms.translation.w);
}