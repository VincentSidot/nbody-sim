alias Particle = vec2<f32>;
alias ParticleExt = vec4<f32>; // (pos.x, pos.y, vel.x, vel.y)
alias Color = vec4<f32>;

@group(0) @binding(0) var<storage, read> position : array<Particle>;
@group(0) @binding(1) var<storage, read> color : array<Color>;

struct VertexShaderOutput {
    @builtin(position) pos: ParticleExt,
    @location(0) color: Color,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexShaderOutput {    
    let p = position[idx];
    let c = color[idx];

    var out: VertexShaderOutput;
    out.pos = ParticleExt(p, 0.0, 1.0);
    out.color = c;

    return out;
}

@fragment
fn fs_main(@location(0) color: Color) -> @location(0) Color {
    return color;
}
