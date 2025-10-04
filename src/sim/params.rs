#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimUniform {
    /// (dt, g, softening, n as f32)
    pub dt_g_soft_n: [f32; 4],
    /// (world_x, world_y, damping, wrap as f32)
    pub world_damp_wrap: [f32; 4],
}

#[derive(Default)]
pub struct SimParams {
    /// Time step
    pub dt: f32,
    /// Gravitational constant
    pub g: f32,
    /// Softening factor to prevent singularities
    pub softening: f32,
    /// Number of particles
    pub n: u32,
    /// Size of the simulation world (width, height)
    pub world: glam::Vec2,
    /// Velocity damping factor
    pub damping: f32,
    /// Whether the world wraps around at the edges
    pub wrap: bool,
}

pub enum ParticleUpdated {
    Less,
    More,
    Same,
}

pub enum ParamsEguiAction {
    /// No action
    None,
    /// Reset the particles
    Reset,
    /// Parameter updates
    ParameterUpdated(ParticleUpdated),
}

impl SimParams {
    pub fn to_uniform(&self) -> SimUniform {
        SimUniform {
            dt_g_soft_n: [self.dt, self.g, self.softening, self.n as f32],
            world_damp_wrap: [
                self.world.x,
                self.world.y,
                self.damping,
                if self.wrap { 1.0 } else { 0.0 },
            ],
        }
    }

    pub fn render_info(&mut self, ui: &mut egui::Ui) -> ParamsEguiAction {
        let mut action = ParamsEguiAction::None;

        ui.label("Simulation Parameters");

        // Time step
        let mut dt = self.dt;
        if ui
            .add(
                egui::Slider::new(&mut dt, 0.001..=0.1)
                    .text("Time Step (dt)")
                    .step_by(0.001)
                    .suffix(" s"),
            )
            .changed()
        {
            self.dt = dt;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Gravitational constant
        let mut g = self.g;
        if ui
            .add(
                egui::Slider::new(&mut g, -50.0..=0.0)
                    .text("Gravitational Constant (g)")
                    .step_by(0.1),
            )
            .changed()
        {
            self.g = g;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Damping factor
        let mut damping = self.damping;
        if ui
            .add(
                egui::Slider::new(&mut damping, 0.9..=1.0)
                    .text("Damping Factor")
                    .step_by(0.001),
            )
            .changed()
        {
            self.damping = damping;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Softening factor
        let mut softening = self.softening;
        if ui
            .add(
                egui::Slider::new(&mut softening, 0.0..=1.0)
                    .text("Softening Factor")
                    .step_by(0.01),
            )
            .changed()
        {
            self.softening = softening;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Number of particles
        let mut n = self.n;
        if ui
            .add(
                egui::Slider::new(&mut n, 1_000..=crate::constants::sim::MAX_PARTICLES)
                    .text("Number of Particles")
                    .step_by(1_000.0),
            )
            .changed()
        {
            if n < self.n {
                action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Less);
            } else if n > self.n {
                action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::More);
            } else {
                action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
            }
            self.n = n;
        }

        // Reset button
        if ui.button("Reset Particles").clicked() {
            action = ParamsEguiAction::Reset;
        }

        action
    }
}
