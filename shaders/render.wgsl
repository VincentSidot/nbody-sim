struct Sim {
  dt_g_soft_n: vec4<f32>,         // (dt, g, softening, n)
  buff_damp_wrap_na: vec4<f32>,   // (buff(0/1), damping, wrap(0/1), na)
  world: vec4<f32>,               // (world.min.x, world.min.y, world.max.x, world.max.y)
};

const PRIMARY_BUFFER: u32 = 0;
const SECONDARY_BUFFER: u32 = 1;

struct Particles {
    pos: array<vec2<f32>>,
};

struct Colors {
    data: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read> position_primary : Particles;
@group(0) @binding(1) var<storage, read> position_secondary : Particles;
@group(0) @binding(2) var<storage, read> color : Colors;
@group(0) @binding(3) var<uniform> S : Sim;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VSOut {
    // Select buffers based on buffer_in_use flag
    let buff = u32(S.buff_damp_wrap_na[0]); // Buffer in use (0 or 1)
    
    var p: vec2<f32>;
    let c = color.data[idx];

    if (buff == PRIMARY_BUFFER) {
        // compute reads primary -> writes secondary -> render reads secondary
        p = position_secondary.pos[idx];
    } else {
        // compute reads secondary -> writes primary -> render reads primary
        p = position_primary.pos[idx];
    }

    var out: VSOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.color = c;

    return out;
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}
