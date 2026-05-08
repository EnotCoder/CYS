struct Uniforms {
    translation: vec4<f32>,
    rotation: vec4<f32>,
    projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

// Вращение вокруг X
fn rotate_x(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x,
        vertex.y * c - vertex.z * s,
        vertex.y * s + vertex.z * c
    );
}

// Вращение вокруг Y
fn rotate_y(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x * c + vertex.z * s,
        vertex.y,
        -vertex.x * s + vertex.z * c
    );
}

// Вращение вокруг Z
fn rotate_z(vertex: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        vertex.x * c - vertex.y * s,
        vertex.x * s + vertex.y * c,
        vertex.z
    );
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    
    // Применяем вращение по всем осям
    var rotated = position;
    rotated = rotate_x(rotated, uniforms.rotation.x);
    rotated = rotate_y(rotated, uniforms.rotation.y);
    rotated = rotate_z(rotated, uniforms.rotation.z);
    
    // Применяем трансляцию
    let world_pos = rotated + uniforms.translation.xyz;
    
    output.position = uniforms.projection * vec4<f32>(world_pos, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(
    @location(0) color: vec3<f32>,
) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}