struct Sim {
  dt_g_soft_n: vec4<f32>,         // (dt, g, softening, n)
  damp_wrap_color_na: vec4<f32>,   // (damping, wrap(0/1), color(0/1), na)
  world: vec4<f32>,               // (world.min.x, world.min.y, world.max.x, world.max.y)
};

const WORKGROUP_SIZE : u32 = __WORKGROUP_SIZE__;

alias Position = vec2<f32>;
alias Velocity = vec2<f32>;
alias Acceleration = vec2<f32>;
alias Color = vec4<f32>;

@group(0) @binding(0) var<storage, read_write> position_write : array<Position>;
@group(0) @binding(1) var<storage, read_write> velocity_write : array<Velocity>;
@group(0) @binding(2) var<storage, read> position_read : array<Position>;
@group(0) @binding(3) var<storage, read> velocity_read : array<Velocity>;
@group(0) @binding(4) var<storage, read_write> color : array<Color>;
@group(0) @binding(5) var<uniform> S : Sim;

fn compute_color(v: Velocity) -> Color {
  let speed = length(v);
  let t = clamp(speed / 1.0, 0.0, 1.0); // assuming max speed ~5 for normalization
  return mix(vec4<f32>(0.0, 0.0, 1.0, 1.0), vec4<f32>(1.0, 0.0, 0.0, 1.0), t); // from blue to red
}

fn fmod(x: f32, y: f32) -> f32 {
    // x - y * floor(x / y)  gives a result in [0, y) when y > 0
    return x - y * floor(x / y);
}

fn clamp_pos(p: Position, world_min: vec2<f32>, world_max: vec2<f32>) -> Position {
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

fn wrap_pos(p: Position, world_min: vec2<f32>, world_max: vec2<f32>) -> Position {
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

fn update_position(p: Position, v: Velocity, dt: f32, world_min: vec2<f32>, world_max: vec2<f32>, wrap: u32) -> Position {
  var np = p + v * dt;
  if (wrap == 0) {
    np = clamp_pos(np, world_min, world_max);
  } else {
    np = wrap_pos(np, world_min, world_max);
  }

  return np;
}

fn compute_single_acceleration(pos: Position, other_pos: Position, g: f32, softening: f32) -> Acceleration {
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

fn compute_acceleration(id: u32, n: u32, g: f32, softening: f32) -> Acceleration {
  var acc = Acceleration(0.0, 0.0);
  let my_pos = position_read[id];
  let softening2 = softening * softening;

  for (var i: u32 = 0u; i < n; i = i + 1u) {
    let pos = position_read[i];
    acc = acc + compute_single_acceleration(my_pos, pos, g, softening2);
  }

  return acc;
}

fn update_velocity(v: Velocity, id: u32, n: u32, g: f32, softening: f32, dt: f32, damp: f32) -> Velocity {
  let acc = compute_acceleration(id, n, g, softening);
  return (v + acc * dt) * damp;
}

@compute @workgroup_size(__WORKGROUP_SIZE__)
fn update(
  @builtin(global_invocation_id) gid : vec3<u32>
) {

  // Compute the id
  let id = gid.x;  
  let n = u32(S.dt_g_soft_n[3]); // Number of particles

  // Bounds check
  if (id >= n) {
    return;
  }

  let inP : Position = position_read[id];
  var inV : Velocity = velocity_read[id];

  // Load parameters
  let dt = S.dt_g_soft_n[0];
  let g = S.dt_g_soft_n[1];
  let softening = S.dt_g_soft_n[2];
  let world_min = S.world.xy;
  let world_max = S.world.zw;
  let damp = S.damp_wrap_color_na[0];
  let wrap = u32(S.damp_wrap_color_na[1]);
  let color_by_speed = u32(S.damp_wrap_color_na[2]);

  let outV = update_velocity(inV, id, n, g, softening, dt, damp);
  let outP = update_position(inP, outV, dt, world_min, world_max, wrap);

  if (color_by_speed == 1u) {
    // Update color based on speed
    let c = compute_color(outV);
    color[id] = c;
  }

  // Store results
  position_write[id] = outP;
  velocity_write[id] = outV;

  return;
}