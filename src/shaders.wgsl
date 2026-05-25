// Структура uniform (только трансляция)
struct Uniforms {
    translation: vec4<f32>,  // x, y, z, w
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Текстура и сэмплер
@group(1) @binding(0)
var my_texture: texture_2d<f32>;

@group(1) @binding(1)
var my_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    
    // Просто добавляем трансляцию к позиции
    let world_pos = position + uniforms.translation.xyz;
    
    // Z слой (чем больше translation.z, тем выше)
    let z_layer = uniforms.translation.z / 100.0;
    
    let size = 0.15;

    output.position = vec4<f32>(world_pos.x * size, world_pos.y * size, z_layer * size, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

@fragment
fn fs_main(
    @location(0) tex_coord: vec2<f32>,
) -> @location(0) vec4<f32> {
    return textureSample(my_texture, my_sampler, tex_coord);
}