struct Sim {
  dt_g_soft_n: vec4<f32>,       // (dt, g, softening, n)
  world_damp_wrap: vec4<f32>,   // (world.x, world.y, damping, wrap(0/1))
  buff_na_na_na: vec4<f32>,       // (buffer_in_use, na, na, na)
};

const WORKGROUP_SIZE : u32 = __WORKGROUP_SIZE__;

alias Particles = array<vec2<f32>>;
alias Velocities = array<vec2<f32>>;

@group(0) @binding(0) var<storage, read_write> P : Particles;
@group(0) @binding(1) var<storage, read_write> V : Velocities;
@group(0) @binding(2) var<uniform> S : Sim;

@compute @workgroup_size(__WORKGROUP_SIZE__)
fn update(
  @builtin(local_invocation_id) local_id : vec3<u32>,
  @builtin(workgroup_id) workgroup_id : vec3<u32>
) {
  // Compute the id
  let id = (workgroup_id * vec3<u32>(WORKGROUP_SIZE, 1, 1) + local_id).x;
  
  let n = u32(S.dt_g_soft_n[3]); // Number of particles

  // Bounds check
  if (id >= n) {
    return;
  }

  // Load parameters
  let dt = S.dt_g_soft_n[0];
  // let g = S.dt_g_soft_n[1];
  // let softening = S.dt_g_soft_n[2];
  // let world = S.world_damp_wrap.xy;
  // let damp = S.world_damp_wrap[2];
  // let wrap = S.world_damp_wrap[3];

  // Currently only update positions regading velocities
  P[id] = P[id] + V[id] * dt;

  return;
}