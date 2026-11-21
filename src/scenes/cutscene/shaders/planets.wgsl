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
@group(1) @binding(4) var sea_tex: texture_2d<f32>;
@group(1) @binding(5) var pixel_sampler: sampler;

const SEA_TILES: f32 = 4.0;

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

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn star_layer(p: vec2<f32>, cells: vec2<f32>, threshold: f32, seed: f32, time: f32) -> f32 {
    let grid = p * cells;
    let cell = floor(grid);
    let f = fract(grid);
    let chance = hash21(cell + vec2<f32>(seed, seed * 1.73));
    if (chance < threshold) {
        return 0.0;
    }
    let offset = vec2<f32>(
        hash21(cell + vec2<f32>(seed + 3.7, 1.3)),
        hash21(cell + vec2<f32>(5.1, seed + 8.9)),
    );
    let radius = 0.06 + 0.07 * hash21(cell + vec2<f32>(seed + 11.0, 7.0));
    let d = length(f - offset);
    let core = 1.0 - smoothstep(0.0, radius, d);
    let glow = 1.0 - smoothstep(radius, radius * 2.4, d);
    let power = 0.45 + 0.55 * hash21(cell + vec2<f32>(seed + 17.0, 2.0));
    let twinkle = 0.80 + 0.20 * sin(time * (0.7 + power * 0.8) + chance * 40.0);
    return (core + glow * 0.22) * power * twinkle;
}

fn starfield(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let p = uv * vec2<f32>(aspect(), 1.0);
    var amount = 0.0;
    amount += star_layer(p, vec2<f32>(38.0), 0.86, 1.0, time) * 0.7;
    amount += star_layer(p, vec2<f32>(78.0), 0.91, 19.0, time) * 0.9;
    amount += star_layer(p, vec2<f32>(140.0), 0.95, 47.0, time) * 1.1;
    return vec3<f32>(0.86, 0.91, 1.0) * amount;
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

fn background(uv: vec2<f32>) -> vec3<f32> {
    let a = aspect();
    let center = vec2<f32>(0.5, 0.5);
    let r = length((uv - center) * vec2<f32>(a, 1.0));
    let nebula = sin(uv.x * 5.0 + sin(uv.y * 4.0 + u.time * 0.03)) * 0.5 + 0.5;
    let base = mix(
        vec3<f32>(0.022, 0.032, 0.070),
        vec3<f32>(0.004, 0.006, 0.020),
        smoothstep(0.05, 0.9, r),
    );
    return base + vec3<f32>(0.018, 0.009, 0.032) * nebula;
}

@fragment
fn fs_solar(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sun_uv = vec2<f32>(0.27, 0.52);
    let earth_uv = vec2<f32>(0.64, 0.46);
    let moon_uv = vec2<f32>(0.79, 0.64);

    var color = background(uv) + starfield(uv, u.time);

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

    // Тайлинг только по горизонтали: по вертикали текстура идёт один раз.
    let sea_uv = vec2<f32>(fract(uv.x * SEA_TILES), uv.y);
    var color = textureSampleLevel(sea_tex, pixel_sampler, sea_uv, 0.0).rgb;

    if (tex_uv.x >= 0.0 && tex_uv.x <= 1.0 && tex_uv.y >= 0.0 && tex_uv.y <= 1.0) {
        let land = textureSampleLevel(continent_tex, pixel_sampler, tex_uv, 0.0);
        let shade = 0.92 + 0.08 * sin(local.x * 12.0 + local.y * 7.0);
        color = mix(color, land.rgb * shade, land.a);
    }

    let cloud = sin(uv.x * 7.0 + sin(uv.y * 5.0 - u.time * 0.08) * 1.6) * 0.5 + 0.5;
    color += vec3<f32>(0.9, 0.95, 1.0) * pow(cloud, 8.0) * 0.035;

    return vec4<f32>(color, 1.0);
}
