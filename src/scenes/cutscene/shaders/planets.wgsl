struct PlanetUniforms {
    resolution: vec2<f32>,
    time: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> u: PlanetUniforms;

@group(1) @binding(0) var sun_tex: texture_2d<f32>;
@group(1) @binding(1) var earth_tex: texture_2d<f32>;
@group(1) @binding(2) var moon_tex: texture_2d<f32>;
@group(1) @binding(3) var continent_tex: texture_2d<f32>;
@group(1) @binding(4) var pixel_sampler: sampler;

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

fn aspect() -> f32 {
    return max(u.resolution.x, 1.0) / max(u.resolution.y, 1.0);
}

fn sample_disc(
    image: texture_2d<f32>,
    uv: vec2<f32>,
    center: vec2<f32>,
    radius: f32,
) -> vec4<f32> {
    let d = (uv - center) * vec2<f32>(aspect(), 1.0) / max(radius, 0.001);
    let r = length(d);
    if (r > 1.03) {
        return vec4<f32>(0.0);
    }
    let tex_uv = d * 0.5 + vec2<f32>(0.5);
    let c = textureSampleLevel(image, pixel_sampler, tex_uv, 0.0);
    let edge = 1.0 - smoothstep(0.99, 1.01, r);
    return vec4<f32>(c.rgb, c.a * edge);
}

/// Мягкий ореол и лучи разной ширины; центр привязан к позиции Солнца.
fn sun_light(uv: vec2<f32>, sun_uv: vec2<f32>, radius: f32, time: f32) -> vec3<f32> {
    let d = (uv - sun_uv) * vec2<f32>(aspect(), 1.0);
    let r = length(d);
    let angle = atan2(d.y, d.x);
    let safe_radius = max(radius, 0.001);

    let halo = exp(-r / safe_radius * 2.6);
    let pattern = 0.5 + 0.5 * sin(angle * 17.0 + 0.6 * sin(angle * 7.0 + time * 0.05));
    let wide = pow(pattern, 10.0) * exp(-r / safe_radius * 1.0);
    let narrow = pow(0.5 + 0.5 * sin(angle * 31.0 - 0.4), 22.0)
        * exp(-r / safe_radius * 0.7);
    let beams = (wide * 0.75 + narrow * 0.45) * smoothstep(0.02, safe_radius, r);
    let pulse = 0.97 + 0.03 * sin(time * 0.8);

    return vec3<f32>(1.0, 0.86, 0.58) * (halo * 0.62 + beams * 0.20) * pulse;
}

fn background(uv: vec2<f32>) -> vec3<f32> {
    let a = aspect();
    let center = vec2<f32>(0.5, 0.5);
    let r = length((uv - center) * vec2<f32>(a, 1.0));
    let nebula = sin(uv.x * 5.0 + sin(uv.y * 4.0 + u.time * 0.03)) * 0.5 + 0.5;
    let base = mix(
        vec3<f32>(0.035, 0.055, 0.115),
        vec3<f32>(0.008, 0.012, 0.032),
        smoothstep(0.05, 0.9, r),
    );
    return base + vec3<f32>(0.025, 0.012, 0.045) * nebula;
}

@fragment
fn fs_solar(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sun_uv = vec2<f32>(0.27, 0.52);
    let earth_uv = vec2<f32>(0.64, 0.46);
    let moon_uv = vec2<f32>(0.79, 0.64);

    var color = background(uv);
    color += sun_light(uv, sun_uv, 0.34, u.time);

    let sun = sample_disc(sun_tex, uv, sun_uv, 0.19);
    color = mix(color, sun.rgb, sun.a);

    // Луна рисуется после Солнца, но до Земли — диски остаются читаемыми.
    let moon = sample_disc(moon_tex, uv, moon_uv, 0.052);
    color = mix(color, moon.rgb * 0.86, moon.a);

    let earth = sample_disc(earth_tex, uv, earth_uv, 0.105);
    let earth_d = length((uv - earth_uv) * vec2<f32>(aspect(), 1.0)) / 0.105;
    let atmosphere = (1.0 - smoothstep(0.88, 1.0, earth_d))
        * smoothstep(0.78, 0.94, earth_d);
    color += vec3<f32>(0.12, 0.35, 0.75) * atmosphere * 0.45;
    color = mix(color, earth.rgb, earth.a);

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_continent(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let a = aspect();
    let center = vec2<f32>(0.5, 0.51);
    let scale = 0.66;
    let local = (uv - center) * vec2<f32>(a, 1.0) / scale;
    let tex_uv = local * 0.5 + vec2<f32>(0.5);

    let wave = sin((uv.y + uv.x * 0.35) * 90.0 + u.time * 1.2) * 0.5 + 0.5;
    let ocean = mix(
        vec3<f32>(0.025, 0.105, 0.19),
        vec3<f32>(0.055, 0.19, 0.30),
        smoothstep(0.0, 1.0, uv.y) * 0.6 + wave * 0.12,
    );
    var color = ocean;

    if (tex_uv.x >= 0.0 && tex_uv.x <= 1.0 && tex_uv.y >= 0.0 && tex_uv.y <= 1.0) {
        let land = textureSampleLevel(continent_tex, pixel_sampler, tex_uv, 0.0);
        let shade = 0.92 + 0.08 * sin(local.x * 12.0 + local.y * 7.0);
        color = mix(color, land.rgb * shade, land.a);
    }

    // Метка магазина: совпадает с фокусом рывка в timeline.rs.
    let pin = vec2<f32>(0.56, 0.47);
    let pd = length((uv - pin) * vec2<f32>(a, 1.0));
    let ring = smoothstep(0.004, 0.008, pd) * (1.0 - smoothstep(0.018, 0.024, pd));
    let pulse = 0.6 + 0.4 * sin(u.time * 3.0);
    color += vec3<f32>(1.0, 0.78, 0.35) * ring * pulse * 0.7;

    let cloud = sin(uv.x * 7.0 + sin(uv.y * 5.0 - u.time * 0.08) * 1.6) * 0.5 + 0.5;
    color += vec3<f32>(0.9, 0.95, 1.0) * pow(cloud, 8.0) * 0.035;

    return vec4<f32>(color, 1.0);
}
