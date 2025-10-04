// shaders/nbody.wgsl (extrait pour uniform packing)
struct Sim {
  dt_g_soft_n: vec4<f32>;       // (dt, g, softening, n)
  world_damp_wrap: vec4<f32>;   // (world.x, world.y, damping, wrap(0/1))
};
@group(0) @binding(0) var<storage, read_write> P : Particles;
@group(0) @binding(1) var<storage, read_write> V : Velocities;
@group(0) @binding(2) var<uniform> S : Sim;
