struct Uniforms {
    translation: vec4<f32>,
    projection: mat4x4<f32>,  // Добавляем матрицу проекции
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    
    // Добавляем трансляцию к позиции
    let world_position = vec4<f32>(
        position.x + uniforms.translation.x,
        position.y + uniforms.translation.y,
        position.z + uniforms.translation.z,
        1.0
    );
    
    // Применяем проекцию
    output.position = uniforms.projection * world_position;
    output.color = color;
    return output;
}

@fragment
fn fs_main(
    @location(0) color: vec3<f32>,
) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}