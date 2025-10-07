struct Sim {
  dt_g_soft_n: vec4<f32>,         // (dt, g, softening, n)
  damp_wrap_color_bootstrap: vec4<f32>,   // (damping, wrap(0/1), color(0/1), bootstrap(0/1))
  world: vec4<f32>,               // (world.min.x, world.min.y, world.max.x, world.max.y)
};

const WORKGROUP_SIZE : u32 = __WORKGROUP_SIZE__; // Set at compile time
const TILE : u32 = WORKGROUP_SIZE;

var<workgroup> pos_tile : array<Position, TILE>;

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
  if (wrap == 0u) {
    np = clamp_pos(np, world_min, world_max);
  } else {
    np = wrap_pos(np, world_min, world_max);
  }

  return np;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn update(
  @builtin(global_invocation_id) gid: vec3<u32>,
  @builtin(local_invocation_id)  lid: vec3<u32>
) {

  // Compute the id
  let id = gid.x;  
  let n = u32(S.dt_g_soft_n[3]); // Number of particles

  // Bounds check
  if (id >= n) { return; }

  let inP : Position = position_read[id];

  // Load parameters
  let dt = S.dt_g_soft_n[0];
  let g = S.dt_g_soft_n[1];
  let soft = S.dt_g_soft_n[2];
  let world_min = S.world.xy;
  let world_max = S.world.zw;
  let damp = S.damp_wrap_color_bootstrap[0];
  let wrap = u32(S.damp_wrap_color_bootstrap[1]);
  let cspd = S.damp_wrap_color_bootstrap[2]; // 0 or 1
  let bootstrap = u32(S.damp_wrap_color_bootstrap[3]); // 0 or 1

  var acc : Acceleration = Acceleration(0.0, 0.0);
  var base : u32 = 0u;

  let soft2 = soft * soft;
  
  loop {
    if (base >= n) { break; }

    let j = base + lid.x;
    if (j < n) {
      pos_tile[lid.x] = position_read[j];   // one coalesced load per lane
    }
    workgroupBarrier();

    let count = min(TILE, n - base);
    for (var k: u32 = 0u; k < count; k = k + 1u) {
      // (optional) skip self when j==id if base+k==id
      let other = pos_tile[k];
      let delta = other - inP;
      let dist2 = dot(delta, delta) + soft2; // add softening term to avoid singularity
      let invd  = inverseSqrt(dist2);
      let invd3 = invd * invd * invd;
      acc += g * delta * invd3;
    }
    workgroupBarrier();

    base += TILE;
  }


  var v_half = velocity_read[id];   // if bootstrap: this is v0; else: v_{n-1/2}

  if (bootstrap == 1u) {
    v_half = v_half + acc * (0.5 * dt);   // one-time half-kick: v_{+1/2} from v0
  } else {
    v_half = v_half + acc * dt;           // normal leapfrog kick
  }
  v_half *= damp;

  let p_new = update_position(inP, v_half, dt, world_min, world_max, wrap);

  // Update color based on speed
  let c = compute_color(v_half);
  color[id] = mix(color[id], c, cspd); // 0 or 1

  // Store results
  position_write[id] = p_new;
  velocity_write[id] = v_half;

  return;
}