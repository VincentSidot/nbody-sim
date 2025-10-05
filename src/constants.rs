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

    pub const PRESENT_MODE_PREFERENCES: &[wgpu::PresentMode] = &[
        wgpu::PresentMode::Fifo,        // Vsync, no tearing
        wgpu::PresentMode::FifoRelaxed, // Low latency, may tear
        wgpu::PresentMode::Mailbox,     // Low latency, no tearing
        wgpu::PresentMode::Fifo,        // Vsync, no tearing
        wgpu::PresentMode::Immediate,   // Low latency, may tear
    ];
}

pub mod egui {
    pub const BORDER_RADIUS: egui::CornerRadius = egui::CornerRadius::same(2);
    pub const SHADOW: egui::epaint::Shadow = egui::epaint::Shadow::NONE;

    /// Number of frames to average for time info (FPS, frame time) this will smooth out the values
    pub const TIME_INFO_LAST_N: usize = 5;
}

pub mod sim {
    use glam::Vec2;

    pub const MAX_PARTICLES: u32 = 1_000_000;
    pub const INITIAL_PARTICLES: u32 = 100_000;

    pub const WORLD_SIZE: Vec2 = Vec2::new(2.0, 2.0);
    pub const DAMPING: f32 = 0.999;
    pub const WRAP: bool = true;
}

pub mod shader {
    pub const WORKGROUP_SIZE: u32 = 64;
    pub const WORKGROUP_SIZE_PAYLOAD: &str = "__WORKGROUP_SIZE__";
}
