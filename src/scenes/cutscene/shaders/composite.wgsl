struct CompositeUniforms {
    // xy: стартовый центр A, z: стартовый zoom A, w: конечный zoom A.
    a: vec4<f32>,
    // xy: конечный центр B, z: конечный zoom B, w: прогресс рывка.
    b: vec4<f32>,
    // x: aspect, y: максимальный смаз, z: fade, w: time.
    misc: vec4<f32>,
    // xy: точка приближения в A, zw: стартовый центр B.
    focus: vec4<f32>,
    // x: стартовый zoom B.
    extra: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: CompositeUniforms;
@group(1) @binding(0) var image_a: texture_2d<f32>;
@group(1) @binding(1) var sampler_a: sampler;
@group(2) @binding(0) var image_b: texture_2d<f32>;
@group(2) @binding(1) var sampler_b: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex: u32) -> VertexOutput {
    var points = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = points[vertex];
    var out: VertexOutput;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}

fn transform_uv(uv: vec2<f32>, center: vec2<f32>, zoom: f32) -> vec2<f32> {
    return center + (uv - vec2<f32>(0.5)) / max(zoom, 0.001);
}

fn radial_sample(
    image: texture_2d<f32>,
    samp: sampler,
    uv: vec2<f32>,
    center: vec2<f32>,
    strength: f32,
) -> vec4<f32> {
    var sum = vec4<f32>(0.0);
    for (var i = 0u; i < 12u; i = i + 1u) {
        let t = f32(i) / 11.0 - 0.5;
        let p = center + (uv - center) * (1.0 + t * strength);
        sum += textureSampleLevel(image, samp, p, 0.0);
    }
    return sum / 12.0;
}

fn finish(color_in: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let aspect = max(u.misc.x, 0.001);
    let d = length((uv - vec2<f32>(0.5)) * vec2<f32>(aspect, 1.0));
    let vignette = 1.0 - 0.40 * smoothstep(0.22, 0.95, d);
    let color = color_in * vignette * u.misc.z;
    return vec4<f32>(max(color, vec3<f32>(0.0)), 1.0);
}

@fragment
fn fs_blit(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_a = transform_uv(in.uv, u.a.xy, u.a.z);
    let center = textureSampleLevel(image_a, sampler_a, uv_a, 0.0).rgb;
    return finish(center, in.uv);
}

@fragment
fn fs_transition(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = clamp(u.b.w, 0.0, 1.0);
    let ta = clamp(p * 2.0, 0.0, 1.0);
    let tb = clamp((p - 0.5) * 2.0, 0.0, 1.0);
    let ea = ta * ta;
    let eb = 1.0 - pow(1.0 - tb, 3.0);

    let zoom_a = mix(u.a.z, u.a.w, ea);
    let center_a = mix(u.a.xy, u.focus.xy, ea);
    let uv_a = transform_uv(in.uv, center_a, zoom_a);

    let zoom_b = mix(u.extra.x, u.b.z, eb);
    let center_b = mix(u.focus.zw, u.b.xy, eb);
    let uv_b = transform_uv(in.uv, center_b, zoom_b);

    let blur = u.misc.y * sin(p * 3.14159265);
    let sample_a = radial_sample(image_a, sampler_a, uv_a, u.focus.xy, blur);
    let sample_b = radial_sample(image_b, sampler_b, uv_b, u.focus.zw, blur);

    // Короткая подмена кадров — не длинный кроссфейд.
    var color = mix(sample_a.rgb, sample_b.rgb, smoothstep(0.44, 0.56, p));
    let flash = exp(-pow((p - 0.5) * 15.0, 2.0)) * 0.16;
    color += vec3<f32>(1.0, 0.9, 0.72) * flash;
    return finish(color, in.uv);
}
