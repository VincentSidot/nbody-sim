pub mod window {
    pub const TITLE: &str = "Particle Playground";
}

pub mod gpu {
    pub const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub const DESIRED_MAXIMUM_FRAME_LATENCY: u32 = 2;
    pub const MSAA_SAMPLES: u32 = 1;
    pub const DITHERING: bool = true;
}

pub mod egui {
    pub const BORDER_RADIUS: egui::CornerRadius = egui::CornerRadius::same(2);
    pub const SHADOW: egui::epaint::Shadow = egui::epaint::Shadow::NONE;
}

pub mod sim {
    use glam::Vec2;

    pub const MAX_PARTICLES: u32 = 1_000_000;
    pub const INITIAL_PARTICLES: u32 = 100_000;

    pub const WORLD_SIZE: Vec2 = Vec2::new(2.0, 2.0);
    pub const DAMPING: f32 = 0.999;
    pub const WRAP: bool = true;
}
