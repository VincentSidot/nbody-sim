// shaders/render.wgsl (lecture positions)
struct Particles {
    pos: array<vec2<f32>>,
};

struct Colors {
    data: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read> P : Particles;
@group(0) @binding(1) var<storage, read> C : Colors;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VSOut {
    var out: VSOut;
    let p = P.pos[idx];
    
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.color = C.data[idx];

    return out;
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}
