#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimUniform {
    /// (dt, g, softening, n as f32)
    pub dt_g_soft_n: [f32; 4],
    /// (world_x, world_y, damping, wrap as f32)
    pub world_damp_wrap: [f32; 4],
    /// Currently used buffer (0 or 1)
    pub buff_na_na_na: [f32; 4],
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferInUse {
    #[default]
    Primary = 0,
    Secondary = 1,
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
    /// Whether the simulation is paused
    pub paused: bool,
    /// Note the current used buffer (0 or 1)
    pub buffer_in_use: BufferInUse,
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
    /// Step the simulation (if paused)
    Step,
}

/// Compute average frame time and FPS over the last N frames and update the last_frames array
///
/// # Arguments
///
/// * `last_frames` - A mutable reference to an array of the last N frame timestamps
///
/// # Returns
///
/// A tuple containing the average frame time in milliseconds and the frames per second (FPS)
fn compute_time_info(
    last_frames: &mut [std::time::Instant; crate::constants::egui::TIME_INFO_LAST_N],
) -> (f32, f32) {
    let now = std::time::Instant::now();
    let mut total_time = 0.0;
    for i in 1..last_frames.len() {
        total_time += (last_frames[i] - last_frames[i - 1]).as_secs_f32();
    }
    total_time += (now - last_frames[0]).as_secs_f32();

    let avg_frame_time = total_time / (last_frames.len() as f32);
    let fps = if avg_frame_time > 0.0 {
        1.0 / avg_frame_time
    } else {
        0.0
    };

    // Update the last_frames array in a circular manner
    // Is this the most efficient way? Probably not, but it's simple and the array is small
    for i in (1..last_frames.len()).rev() {
        last_frames[i] = last_frames[i - 1];
    }
    last_frames[0] = now;

    (avg_frame_time * 1000.0, fps) // return in ms and fps
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
            buff_na_na_na: [
                match self.buffer_in_use {
                    BufferInUse::Primary => 0.0,
                    BufferInUse::Secondary => 1.0,
                },
                0.0,
                0.0,
                0.0,
            ],
        }
    }

    pub fn render_info(
        &mut self,
        ui: &mut egui::Ui,
        last_frame: &mut [std::time::Instant; crate::constants::egui::TIME_INFO_LAST_N],
    ) -> ParamsEguiAction {
        let mut action = ParamsEguiAction::None;

        ui.heading("Simulation Info");
        // Display the current frame time
        let (frame_time, frame_per_sec) = compute_time_info(last_frame);
        ui.label(format!("Frame Time: {:.2} ms", frame_time));
        ui.label(format!("FPS: {:.2}", frame_per_sec));

        ui.separator();
        ui.heading("Simulation Parameters");

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

        ui.separator();

        ui.horizontal(|ui| {
            // Reset button
            if ui.button("Reset Particles").clicked() {
                action = ParamsEguiAction::Reset;
            }

            // Pause/Resume button
            if ui
                .button(if self.paused { "Resume" } else { "Pause" })
                .clicked()
            {
                self.paused = !self.paused;
                action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
            }

            // Step button (only enabled when paused)
            if ui
                .add_enabled(self.paused, egui::Button::new("Step"))
                .clicked()
            {
                action = ParamsEguiAction::Step;
            }
        });

        action
    }
}
