struct Sim {
  dt_g_soft_n: vec4<f32>,       // (dt, g, softening, n)
  world_damp_wrap: vec4<f32>,   // (world.x, world.y, damping, wrap(0/1))
  buff_na_na_na: vec4<f32>,       // (buffer_in_use, na, na, na)
};

struct Particles {
    pos: array<vec2<f32>>,
};

struct Colors {
    data: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read> P : Particles;
@group(0) @binding(1) var<storage, read> C : Colors;
@group(0) @binding(2) var<uniform> S : Sim;

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
