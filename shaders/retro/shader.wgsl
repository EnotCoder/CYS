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

// ----------------------------------------
// Настройки CRT.
// ----------------------------------------

const CRT_SCANLINE_STRENGTH: f32 = 0.6;
const CRT_SCANLINE_PERIOD: f32 = 3.0;

const CRT_MASK_STRENGTH: f32 = 0.12;
const CRT_NOISE_STRENGTH: f32 = 0.025;

const CRT_SATURATION: f32 = 0.85;
const CRT_CONTRAST: f32 = 1.06;
const CRT_BRIGHTNESS: f32 = 1.08;

// ----------------------------------------
// Вращение.
// ----------------------------------------

fn rotate_x(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);

    return vec3<f32>(
        vertex.x,
        vertex.y * c - vertex.z * s,
        vertex.y * s + vertex.z * c
    );
}

fn rotate_y(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);

    return vec3<f32>(
        vertex.x * c + vertex.z * s,
        vertex.y,
        -vertex.x * s + vertex.z * c
    );
}

fn rotate_z(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);

    return vec3<f32>(
        vertex.x * c - vertex.y * s,
        vertex.x * s + vertex.y * c,
        vertex.z
    );
}

// ----------------------------------------
// Статичный шум.
// ----------------------------------------

fn crt_noise(pixel: vec2<f32>) -> f32 {
    return fract(
        sin(dot(pixel, vec2<f32>(12.9898, 78.233)))
        * 43758.5453
    );
}

// ----------------------------------------
// Вершинный шейдер.
// ----------------------------------------

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;

    var rotated = position;

    rotated = rotate_x(rotated, uniforms.rotation.x);
    rotated = rotate_y(rotated, uniforms.rotation.y);
    rotated = rotate_z(rotated, uniforms.rotation.z);

    let world_pos = rotated + uniforms.translation.xyz;

    let size = 0.223 * size_uniform.map_size;

    let cam_x =
        (world_pos.x - size_uniform.offset_x)
        * size
        / size_uniform.aspect;

    let cam_y =
        (world_pos.y - size_uniform.offset_y)
        * size;

    output.position = vec4<f32>(cam_x, cam_y, 0.0, 1.0);
    output.tex_coord = tex_coord;
    output.world_pos = world_pos;

    return output;
}

// ----------------------------------------
// Фрагментный шейдер.
// ----------------------------------------

@fragment
fn fs_main(
    @builtin(position) screen_pos: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
) -> @location(0) vec4<f32> {
    let color = textureSample(
        my_texture,
        my_sampler,
        tex_coord
    );

    if (color.a < 0.01) {
        discard;
    }

    // ----------------------------------------
    // Освещение.
    // ----------------------------------------

    let ambient_factor =
        1.0 - size_uniform.night_factor * 0.8;

    var final_light = vec3<f32>(ambient_factor);

    for (
        var i: u32 = 0u;
        i < size_uniform.light_count;
        i = i + 1u
    ) {
        let light = lights[i];

        let dist = distance(
            world_pos.xy,
            light.position.xy
        );

        let radius = max(light.radius, 0.0001);

        if (dist < radius) {
            let atten = pow(
                1.0 - dist / radius,
                2.0
            );

            final_light +=
                light.color.rgb
                * light.color.a
                * atten;
        }
    }

    final_light = min(
        final_light,
        vec3<f32>(1.5)
    );

    var rgb = color.rgb * final_light;

    // ----------------------------------------
    // Слегка выцветшие цвета.
    // ----------------------------------------

    let luma = dot(
        rgb,
        vec3<f32>(0.2126, 0.7152, 0.0722)
    );

    rgb = mix(
        vec3<f32>(luma),
        rgb,
        CRT_SATURATION
    );

    // Лёгкий тёплый оттенок.
    rgb *= vec3<f32>(1.03, 1.0, 0.96);

    rgb =
        (rgb - vec3<f32>(0.5))
        * CRT_CONTRAST
        + vec3<f32>(0.5);

    // ----------------------------------------
    // Горизонтальные строки.
    // ----------------------------------------

    let scan_wave = 0.5 + 0.5 * cos(
        screen_pos.y
        * 6.28318530718
        / CRT_SCANLINE_PERIOD
    );

    let scanline =
        1.0 - CRT_SCANLINE_STRENGTH * scan_wave;

    rgb *= scanline;

    // ----------------------------------------
    // RGB-маска.
    // ----------------------------------------

    let column = u32(floor(screen_pos.x)) % 3u;

    var phosphor = vec3<f32>(
        1.0 - CRT_MASK_STRENGTH
    );

    if (column == 0u) {
        phosphor.r = 1.0;
    } else if (column == 1u) {
        phosphor.g = 1.0;
    } else {
        phosphor.b = 1.0;
    }

    rgb *= phosphor;

    // ----------------------------------------
    // Статичное зерно.
    // ----------------------------------------

    let noise =
        crt_noise(floor(screen_pos.xy)) - 0.5;

    rgb += vec3<f32>(
        noise * CRT_NOISE_STRENGTH
    );

    // ----------------------------------------
    // Итоговая яркость и прозрачность.
    // ----------------------------------------

    rgb *= CRT_BRIGHTNESS;

    rgb = clamp(
        rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

    return vec4<f32>(
        rgb,
        color.a * uniforms.translation.w
    );
}