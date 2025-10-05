mod params;

use glam::Vec2;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub use params::{BufferInUse, ParamsEguiAction, ParticleUpdated, SimParams, SimUniform};

pub fn reset_galaxy(n: u32) -> (Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<[f32; 4]>) {
    let mut rng = StdRng::seed_from_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );
    let half = n / 2;
    let mut pos = Vec::with_capacity(n as usize);
    let mut vel = Vec::with_capacity(n as usize);
    let mut col = Vec::with_capacity(n as usize);

    let mut make_disc = |count: u32,
                         center: Vec2,
                         rot_dir: f32,
                         pos: &mut Vec<[f32; 2]>,
                         vel: &mut Vec<[f32; 2]>,
                         col: &mut Vec<[f32; 4]>| {
        for _ in 0..count {
            // rayon ~ uniform in disc
            let r = (rng.random::<f32>().sqrt()) * 0.45; // compact
            let theta = rng.random::<f32>() * std::f32::consts::TAU;
            let p = center + Vec2::new(theta.cos(), theta.sin()) * r;

            // v = tangente * vmag
            let tangent = Vec2::new(-theta.sin(), theta.cos()) * rot_dir;
            let vmag = 0.35 / (r + 0.02).sqrt();
            let v = tangent * vmag;

            // color (red-blueish gradient relative to center with some noise)
            let mut red_component =
                ((p.x - center.x) / 0.5 + 1.0) * 0.5 + rng.random::<f32>() * 0.1;
            let mut blue_component =
                ((-p.x + center.x) / 0.5 + 1.0) * 0.5 + rng.random::<f32>() * 0.1;
            // clamp to [0,1]
            red_component = red_component.clamp(0.0, 1.0);
            blue_component = blue_component.clamp(0.0, 1.0);

            pos.push([p.x, p.y]);
            vel.push([v.x, v.y]);
            col.push([red_component, blue_component, 1.0, 1.0]);
        }
    };

    make_disc(
        half,
        Vec2::new(-0.35, 0.0),
        1.0,
        &mut pos,
        &mut vel,
        &mut col,
    );
    make_disc(
        n - half,
        Vec2::new(0.35, 0.0),
        -1.0,
        &mut pos,
        &mut vel,
        &mut col,
    );

    (pos, vel, col)
}
