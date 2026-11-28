struct HintUniforms {
    rect: vec4<f32>,
    style: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: HintUniforms;
@group(1) @binding(0) var hint_tex: texture_2d<f32>;
@group(1) @binding(1) var linear_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    let corner = corners[vertex];
    let uv = u.rect.xy + corner * u.rect.zw;
    var out: VertexOutput;
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let text = textureSampleLevel(hint_tex, linear_sampler, in.uv, 0.0);
    let pulse = 0.72 + 0.18 * sin(u.style.y * 2.2);
    let alpha = text.a * u.style.x * pulse;
    return vec4<f32>(text.rgb * alpha, alpha);
}
