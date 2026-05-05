struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @location(0) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

// Uniform-буфер с позицией
struct Uniforms {
    translation: vec2<f32>,
};

// Добавляем привязку uniform буфера
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // ПРИМЕНЯЕМ СМЕЩЕНИЕ К ПОЗИЦИИ
    let pos = in.position + uniforms.translation;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.color = vec4<f32>(in.color, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}