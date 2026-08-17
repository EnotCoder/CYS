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
    night_factor: f32,
    light_count: u32,
    _padding: vec2<f32>,
};

struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
    radius: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0)
var<storage, read> uniforms: Uniforms;

// Текстура и сэмплер
@group(1) @binding(0)
var my_texture: texture_2d<f32>;

@group(1) @binding(1)
var my_sampler: sampler;

@group(2) @binding(0)
var<uniform> size_uniform: Size;

@group(2) @binding(1)
var<storage, read> lights: array<Light>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
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
    output.world_pos = world_pos;
    return output;
}

@fragment
fn fs_main(
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, tex_coord);
    if (color.a < 0.01) { discard; }

    let ambient_factor = 1.0 - size_uniform.night_factor * 0.8;
    var final_light = vec3<f32>(ambient_factor);

    // Итерируемся по источникам света
    for (var i: u32 = 0u; i < size_uniform.light_count; i = i + 1u) {
        let light = lights[i];
        let dist = distance(world_pos.xy, light.position.xy);
        
        if (dist < light.radius) {
            // Мягкое затухание (инвертированный квадрат расстояния)
            let atten = pow(1.0 - dist / light.radius, 2.0);
            final_light += light.color.rgb * light.color.a * atten;
        }
    }

    // Ограничиваем свет, так как у нас теперь SDR (без Bloom)
    final_light = min(final_light, vec3<f32>(1.5));

    return vec4<f32>(color.rgb * final_light, color.a * uniforms.translation.w);
}
