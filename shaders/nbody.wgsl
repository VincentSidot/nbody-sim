struct Sim {
  dt_g_soft_n: vec4<f32>,         // (dt, g, softening, n)
  buff_damp_wrap_color: vec4<f32>,   // (buff(0/1), damping, wrap(0/1), color(0/1))
  world: vec4<f32>,               // (world.min.x, world.min.y, world.max.x, world.max.y)
};

const PRIMARY_BUFFER: u32 = 0;
const SECONDARY_BUFFER: u32 = 1;
const WORKGROUP_SIZE : u32 = __WORKGROUP_SIZE__;

alias Particle = vec2<f32>;
alias Velocity = vec2<f32>;

@group(0) @binding(0) var<storage, read_write> position_primary : array<Particle>;
@group(0) @binding(1) var<storage, read_write> velocity_primary : array<Velocity>;
@group(0) @binding(2) var<storage, read_write> position_secondary : array<Particle>;
@group(0) @binding(3) var<storage, read_write> velocity_secondary : array<Velocity>;
@group(0) @binding(4) var<storage, read_write> color : array<vec4<f32>>;
@group(0) @binding(5) var<uniform> S : Sim;

fn compute_color(v: Velocity) -> vec4<f32> {
  let speed = length(v);
  let t = clamp(speed / 1.0, 0.0, 1.0); // assuming max speed ~5 for normalization
  return mix(vec4<f32>(0.0, 0.0, 1.0, 1.0), vec4<f32>(1.0, 0.0, 0.0, 1.0), t); // from blue to red
}

fn fmod(x: f32, y: f32) -> f32 {
    // x - y * floor(x / y)  gives a result in [0, y) when y > 0
    return x - y * floor(x / y);
}

fn read_pos(i: u32, buff: u32) -> Particle {
  // select(x, y, b) returns b ? y : x
  return select(position_secondary[i], position_primary[i], buff == PRIMARY_BUFFER);
}

fn read_vel(i: u32, buff: u32) -> Velocity {
  return select(velocity_secondary[i], velocity_primary[i], buff == PRIMARY_BUFFER);
}

fn clamp_pos(p: vec2<f32>, world_min: vec2<f32>, world_max: vec2<f32>) -> vec2<f32> {
  var np = p;
  if (np.x < world_min.x) {
    np.x = world_min.x;
  }
  if (np.y < world_min.y) {
    np.y = world_min.y;
  }
  if (np.x > world_max.x) {
    np.x = world_max.x;
  }
  if (np.y > world_max.y) {
    np.y = world_max.y;
  }
  return np;
}

fn wrap_pos(p: vec2<f32>, world_min: vec2<f32>, world_max: vec2<f32>) -> vec2<f32> {
  var np = p;
  let world_size = world_max - world_min;

  if (np.x < world_min.x) {
    np.x = world_max.x - fmod(world_min.x - np.x, world_size.x);
  }
  if (np.y < world_min.y) {
    np.y = world_max.y - fmod(world_min.y - np.y, world_size.y);
  }
  if (np.x > world_max.x) {
    np.x = world_min.x + fmod(np.x - world_max.x, world_size.x);
  }
  if (np.y > world_max.y) {
    np.y = world_min.y + fmod(np.y - world_max.y, world_size.y);
  }
  return np;
}

fn update_position(p: Particle, v: Velocity, dt: f32, world_min: vec2<f32>, world_max: vec2<f32>, wrap: u32) -> vec2<f32> {
  var np = p + v * dt;
  if (wrap == 0) {
    np = clamp_pos(np, world_min, world_max);
  } else {
    np = wrap_pos(np, world_min, world_max);
  }

  return np;
}

fn compute_single_acceleration(pos: Particle, other_pos: Particle, g: f32, softening: f32) -> vec2<f32> {
  let delta = other_pos - pos;
  let dist_sqr = dot(delta, delta) + softening;
  let inv_dist = inverseSqrt(dist_sqr);
  let inv_dist3 = inv_dist * inv_dist * inv_dist;
  // F = G * (m1*m2) / r^2  but m1=m2=1 so F = G/r^2; a = F/m = F
  // a = F/m = F/1 = F
  // ax = g * dx / r^3
  // ay = g * dy / r^3
  return g * delta * inv_dist3; 
}

fn compute_acceleration(id: u32, n: u32, g: f32, softening: f32, buff: u32) -> vec2<f32> {
  var acc = vec2<f32>(0.0, 0.0);
  let my_pos = read_pos(id, buff);
  let softening2 = softening * softening;

  for (var i: u32 = 0u; i < n; i = i + 1u) {
    let pos = read_pos(i, buff);
    acc = acc + compute_single_acceleration(my_pos, pos, g, softening2);
  }

  return acc;
}

fn update_velocity(v: Velocity, id: u32, n: u32, g: f32, softening: f32, dt: f32, damp: f32, buff: u32) -> Velocity {
  let acc = compute_acceleration(id, n, g, softening, buff);
  return (v + acc * dt) * damp;
}

@compute @workgroup_size(__WORKGROUP_SIZE__)
fn update(
  @builtin(global_invocation_id) gid : vec3<u32>
) {

  // Compute the id
  let id = gid.x;  
  let n = u32(S.dt_g_soft_n[3]); // Number of particles
  let buff = u32(S.buff_damp_wrap_color[0]); // Buffer in use (0 or 1)

  // Bounds check
  if (id >= n) {
    return;
  }

  // Select buffers based on buffer_in_use flag
  let inP : Particle = read_pos(id, buff);
  var inV : Velocity = read_vel(id, buff);

  // Load parameters
  let dt = S.dt_g_soft_n[0];
  let g = S.dt_g_soft_n[1];
  let softening = S.dt_g_soft_n[2];
  let world_min = S.world.xy;
  let world_max = S.world.zw;
  let damp = S.buff_damp_wrap_color[1];
  let wrap = u32(S.buff_damp_wrap_color[2]);
  let color_by_speed = u32(S.buff_damp_wrap_color[3]);

  let outV = update_velocity(inV, id, n, g, softening, dt, damp, buff);
  let outP = update_position(inP, outV, dt, world_min, world_max, wrap);

  if (color_by_speed == 1u) {
    // Update color based on speed
    let c = compute_color(outV);
    color[id] = c;
  }

  // Store results
  if (buff == PRIMARY_BUFFER) {
    position_secondary[id] = outP;
    velocity_secondary[id] = outV;
  } else {
    position_primary[id] = outP;
    velocity_primary[id] = outV;  
  }

  return;
}