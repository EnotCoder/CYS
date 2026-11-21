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

@fragment
fn fs_solar(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sun_uv = vec2<f32>(0.27, 0.52);
    let earth_uv = vec2<f32>(0.64, 0.46);
    let moon_uv = vec2<f32>(0.79, 0.64);

    // Звёзды рисуются тем же инстансированным пайплайном до этого прохода.
    // Фон прозрачный, диски — premultiplied alpha, чтобы звёзды просвечивали
    // только вокруг планет.
    var color = vec3<f32>(0.0);
    var alpha = 0.0;

    let sun = sample_disc(sun_tex, uv, sun_uv, 0.19);
    color = mix(color, sun.rgb * sun.a, sun.a);
    alpha = sun.a + alpha * (1.0 - sun.a);

    // Луна рисуется после Солнца, но до Земли — диски остаются читаемыми.
    let moon = sample_disc(moon_tex, uv, moon_uv, 0.052);
    color = mix(color, moon.rgb * moon.a * 0.86, moon.a);
    alpha = moon.a + alpha * (1.0 - moon.a);

    let earth = sample_disc(earth_tex, uv, earth_uv, 0.105);
    let earth_d = length((uv - earth_uv) * vec2<f32>(aspect(), 1.0)) / 0.105;
    let atmosphere = (1.0 - smoothstep(0.88, 1.0, earth_d))
        * smoothstep(0.78, 0.94, earth_d);
    let atmo_a = atmosphere * 0.45 * (1.0 - alpha);
    color += vec3<f32>(0.12, 0.35, 0.75) * atmo_a;
    alpha += atmo_a;
    color = mix(color, earth.rgb * earth.a, earth.a);
    alpha = earth.a + alpha * (1.0 - earth.a);

    return vec4<f32>(color, alpha);
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
