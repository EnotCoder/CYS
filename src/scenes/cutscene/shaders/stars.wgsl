struct StarUniforms {
    target_size: vec2<f32>,
    time: f32,
    visibility: f32,
};

@group(0) @binding(0) var<uniform> u: StarUniforms;

struct StarInput {
    @location(0) center_px: vec2<f32>,
    @location(1) size_px: f32,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, star: StarInput) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5,  0.5),
    );
    let quad = corners[vertex];
    let px = star.center_px + quad * star.size_px;
    let x = px.x / max(u.target_size.x, 1.0) * 2.0 - 1.0;
    let y = 1.0 - px.y / max(u.target_size.y, 1.0) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = quad + vec2<f32>(0.5);
    out.color = star.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(in.uv - vec2<f32>(0.5)) * 2.0;
    let core = 1.0 - smoothstep(0.05, 1.0, d);
    let glow = exp(-d * d * 4.0) * 0.28;
    let alpha = clamp((core + glow) * in.color.a, 0.0, 1.0);
    let rgb = in.color.rgb * alpha;
    return vec4<f32>(rgb, alpha);
}
