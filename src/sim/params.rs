use crate::constants;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// Note the padding to make the struct 16-byte aligned
pub struct SimUniform {
    /// (dt, g, softening, n as f32)
    pub dt_g_soft_n: [f32; 4],
    /// (world_x, world_y, damping, wrap as f32, color_by_speed as f32)
    pub buff_damp_wrap_color: [f32; 4],
    /// Currently used buffer (0 or 1)
    pub world: [f32; 4],
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferInUse {
    #[default]
    Primary = 0,
    Secondary = 1,
}

impl BufferInUse {
    pub fn tick(&mut self) {
        *self = match self {
            BufferInUse::Primary => BufferInUse::Secondary,
            BufferInUse::Secondary => BufferInUse::Primary,
        }
    }
}

pub struct SimParams {
    /// Time step
    pub dt: f32,
    /// Gravitational constant
    pub g: f32,
    /// Softening factor to prevent singularities
    pub softening: f32,
    /// Number of particles
    pub n: u32,
    /// Size of the simulation world (as a square, from -world to +world)
    pub world: [glam::Vec2; 2],
    /// Velocity damping factor
    pub damping: f32,
    /// Whether the world wraps around at the edges
    pub wrap: bool,
    /// Whether the simulation is paused
    pub paused: bool,
    /// Change color based on speed
    pub color_by_speed: bool,
    /// Note the current used buffer (0 or 1)
    pub buffer_in_use: BufferInUse,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            dt: constants::sim::DT,
            g: constants::sim::G,
            damping: constants::sim::DAMPING,
            softening: constants::sim::SOFTENING,
            n: constants::sim::INITIAL_PARTICLES,
            world: constants::sim::WORLD_SIZE,
            wrap: constants::sim::WRAP,
            paused: constants::sim::PAUSED,
            color_by_speed: constants::sim::COLOR_BY_SPEED,
            buffer_in_use: BufferInUse::Primary,
        }
    }
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
            buff_damp_wrap_color: [
                match self.buffer_in_use {
                    BufferInUse::Primary => 0.0,
                    BufferInUse::Secondary => 1.0,
                },
                self.damping,
                if self.wrap { 1.0 } else { 0.0 },
                if self.color_by_speed { 1.0 } else { 0.0 },
            ],
            world: [
                self.world[0].x,
                self.world[0].y,
                self.world[1].x,
                self.world[1].y,
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
                egui::Slider::new(&mut dt, constants::sim::DT_RANGE)
                    .text("Time Step (dt)")
                    .step_by(0.001)
                    .suffix(" s"),
            )
            .on_hover_text("The time step for each simulation update (in seconds)")
            .changed()
        {
            self.dt = dt;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Gravitational constant
        let mut g = self.g;
        if ui
            .add(
                egui::Slider::new(&mut g, constants::sim::G_RANGE)
                    .text("Gravitational Constant (g)")
                    .step_by(constants::sim::G_STEP),
            )
            .on_hover_text("The gravitational constant used in the simulation")
            .changed()
        {
            self.g = g;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Softening factor
        let mut softening = self.softening;
        if ui
            .add(
                egui::Slider::new(&mut softening, constants::sim::SOFTENING_RANGE)
                    .text("Softening Factor")
                    .step_by(constants::sim::SOFTENING_STEP),
            )
            .on_hover_text("Softening factor to prevent singularities in force calculations")
            .changed()
        {
            self.softening = softening;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Damping factor
        let mut damping = self.damping;
        if ui
            .add(
                egui::Slider::new(&mut damping, constants::sim::DAMPING_RANGE)
                    .text("Damping Factor")
                    .step_by(constants::sim::DAMPING_STEP),
            )
            .on_hover_text("Velocity damping factor to simulate friction or drag")
            .changed()
        {
            self.damping = damping;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Number of particles
        let mut n = self.n;
        if ui
            .add(
                egui::Slider::new(&mut n, constants::sim::INITIAL_PARTICLES_RANGE)
                    .text("Number of Particles")
                    .step_by(constants::sim::INITIAL_PARTICLES_STEP),
            )
            .on_hover_text("The total number of particles in the simulation")
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

        // World wrap
        let mut wrap = self.wrap;
        if ui
            .checkbox(&mut wrap, "Wrap World Edges")
            .on_hover_text(
                "Whether particles that exit one side of the world re-enter from the opposite side",
            )
            .changed()
        {
            self.wrap = wrap;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        // Color by speed
        let mut color_by_speed = self.color_by_speed;
        if ui
            .checkbox(&mut color_by_speed, "Color by Speed")
            .on_hover_text("Whether particle color is determined by its speed")
            .changed()
        {
            self.color_by_speed = color_by_speed;
            action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
        }

        ui.separator();

        ui.horizontal(|ui| {
            // Reset parameters button
            if ui.button("Reset Parameters").clicked() {
                let mut _def = SimParams::default();
                _def.n = self.n; // keep current n
                *self = _def;
                action = ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same);
            }

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
