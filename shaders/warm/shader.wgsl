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
// Настройки комиксного стиля.
// ----------------------------------------

// Количество уровней яркости, минимум 2.
const COMIC_LEVELS: f32 = 50.0;

// Насыщенность: 1.0 — исходная.
const COMIC_SATURATION: f32 = 1.30;

// Контраст.
const COMIC_CONTRAST: f32 = 1.10;

// Расстояние между точками в пикселях framebuffer.
const COMIC_DOT_SPACING: f32 = 6.0;

// Насколько сильно точки затемняют картинку.
const COMIC_DOT_STRENGTH: f32 = 0.65;

// Оттенок тёмной краски.
const COMIC_INK: vec3<f32> = vec3<f32>(0.035, 0.025, 0.055);

// Лёгкий оттенок бумаги.
const COMIC_PAPER: vec3<f32> = vec3<f32>(1.0, 0.98, 0.93);

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

    // Производные считаем до discard и условных веток.
    let dot_spacing = max(COMIC_DOT_SPACING, 1.0);

    // Поворот точечной сетки на 45 градусов.
    let rotated_screen = vec2<f32>(
        screen_pos.x + screen_pos.y,
        screen_pos.y - screen_pos.x
    ) * 0.70710678118;

    let grid_pos = rotated_screen / dot_spacing;

    // Координаты внутри одной ячейки: -0.5 .. 0.5.
    let cell_pos = fract(grid_pos) - vec2<f32>(0.5);
    let dot_distance = length(cell_pos);

    // Ширина сглаживания края точки.
    let dot_aa = max(length(fwidth(grid_pos)) * 0.5, 0.001);

    if (color.a < 0.01) {
        discard;
    }

    // ----------------------------------------
    // Исходное освещение.
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
    // Насыщенность и контраст.
    // ----------------------------------------

    let original_luma = dot(
        rgb,
        vec3<f32>(0.2126, 0.7152, 0.0722)
    );

    rgb = mix(
        vec3<f32>(original_luma),
        rgb,
        COMIC_SATURATION
    );

    rgb =
        (rgb - vec3<f32>(0.5))
        * COMIC_CONTRAST
        + vec3<f32>(0.5);

    rgb = clamp(
        rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

    // Яркость до постеризации — для размера точек.
    let luma = dot(
        rgb,
        vec3<f32>(0.2126, 0.7152, 0.0722)
    );

    // ----------------------------------------
    // Постеризация яркости.
    // Сохраняем соотношение цветовых каналов.
    // ----------------------------------------

    let levels = max(COMIC_LEVELS, 2.0);
    let steps = levels - 1.0;

    let poster_luma = floor(luma * steps + 0.5) / steps;

    rgb *= poster_luma / max(luma, 0.0001);

    rgb = clamp(
        rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );

    // ----------------------------------------
    // Растровые точки в тенях.
    // Чем темнее участок, тем крупнее точки.
    // ----------------------------------------

    let shadow = 1.0 - smoothstep(0.10, 0.85, luma);

    let dot_radius = 0.42 * sqrt(shadow);

    let dot_mask = (
        1.0 - smoothstep(
            dot_radius - dot_aa,
            dot_radius + dot_aa,
            dot_distance
        )
    ) * shadow;

    rgb = mix(
        rgb,
        COMIC_INK,
        clamp(dot_mask * COMIC_DOT_STRENGTH, 0.0, 1.0)
    );

    // Лёгкий оттенок печати на бумаге.
    rgb *= COMIC_PAPER;

    return vec4<f32>(
        clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)),
        color.a * uniforms.translation.w
    );
}