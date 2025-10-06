mod params;

use glam::Vec2;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub use params::{BufferInUse, ParamsEguiAction, ParticleUpdated, SimParams, SimUniform};

fn color(r: u8, g: u8, b: u8) -> [f32; 3] {
    [(r as f32) / 255.0, (g as f32) / 255.0, (b as f32) / 255.0]
}

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
                         start_color: &[f32; 3],
                         end_color: &[f32; 3],
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
            let vmag = 0.035 / (r + 0.02).sqrt();
            let v = tangent * vmag * (0.9 + rng.random::<f32>() * 0.1); // add some noise (10%)

            // color (gradient from start_color to end_color regarding the distance to the center)
            // pow3 for better contrast
            let red_component =
                start_color[0] + (end_color[0] - start_color[0]) * (r / 0.45).powi(3);
            let blue_component =
                start_color[2] + (end_color[2] - start_color[2]) * (r / 0.45).powi(3);
            let green_component =
                start_color[1] + (end_color[1] - start_color[1]) * (r / 0.45).powi(3);

            pos.push([p.x, p.y]);
            vel.push([v.x, v.y]);
            col.push([red_component, green_component, blue_component, 1.0]); // RGBA
        }
    };

    make_disc(
        half,
        Vec2::new(-0.35, 0.0),
        1.0,
        &color(255, 128, 0),  // orange core
        &color(65, 105, 225), // royal blue outskirts
        &mut pos,
        &mut vel,
        &mut col,
    );
    make_disc(
        n - half,
        Vec2::new(0.35, 0.0),
        -1.0,
        &color(0, 165, 225), // light blue core
        &color(123, 104, 0), // dark goldenrod outskirts
        &mut pos,
        &mut vel,
        &mut col,
    );

    (pos, vel, col)
}
