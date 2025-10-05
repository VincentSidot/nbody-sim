mod buffers;
mod compute;
mod egui_renderer;
mod renderer;

pub use egui_renderer::EguiRenderer;

use std::sync::Arc;

use winit::window::Window;

use crate::{
    constants,
    gpu::buffers::GpuBuffers,
    sim::{self, BufferInUse, ParamsEguiAction, ParticleUpdated, SimParams, reset_galaxy},
};

pub struct State {
    // WGPU core components
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Winit winwow
    window: Arc<Window>,

    // Optional egui renderer
    egui: Option<EguiRenderer>,

    // Surface formats
    srgb_format: wgpu::TextureFormat,
    unorm_format: wgpu::TextureFormat,

    // Render pipeline
    render_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,

    /// Compute pipeline
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,

    /// Buffers
    buffers: buffers::GpuBuffers,

    // Simulation state
    params: sim::SimParams,

    // State information
    last_frame: [std::time::Instant; constants::egui::TIME_INFO_LAST_N],
}

impl State {
    pub async fn new(window: Arc<Window>, enable_egui: bool) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        // Try to find a srgb format, if not fallback to the first available
        let srgb_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        // Derive a matching UNORM (linear) format for egui
        let unorm_format = match srgb_format {
            wgpu::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8Unorm,
            _ => wgpu::TextureFormat::Bgra8Unorm, // reasonable fallback
        };

        // Try to find a present mode that supports low latency and vsync
        // Fallback to the first available mode if not found
        let mut present_mode = surface_caps.present_modes[0];
        for &preferred in constants::gpu::PRESENT_MODE_PREFERENCES {
            if surface_caps.present_modes.contains(&preferred) {
                present_mode = preferred;
                break;
            }
        }
        log::trace!("Chosen present mode: {:?}", present_mode);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: srgb_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![unorm_format],
            desired_maximum_frame_latency: constants::gpu::DESIRED_MAXIMUM_FRAME_LATENCY,
        };

        // Configure the surface with the chosen format and size
        surface.configure(&device, &config);

        let egui = if enable_egui {
            Some(EguiRenderer::new(
                &device,
                unorm_format,
                None, // No depth format
                constants::gpu::MSAA_SAMPLES,
                constants::gpu::DITHERING,
                &window,
            ))
        } else {
            None
        };

        let params = SimParams::default();
        let buffers = GpuBuffers::create(&device, params.n);

        let render_shader = renderer::make_shader(&device);
        let render_bind_group_layout = renderer::make_bind_group_layout(&device);
        let render_pipeline_layout =
            renderer::make_pipeline_layout(&device, &[&render_bind_group_layout]);
        let render_pipeline = renderer::make_pipeline(
            &device,
            &render_pipeline_layout,
            &render_shader,
            srgb_format,
        );
        let render_bind_group =
            renderer::make_bind_group(&device, &render_bind_group_layout, &buffers);

        let compute_shader = compute::make_shader(&device);
        let compute_bind_group_layout = compute::make_bind_group_layout(&device);
        let compute_pipeline_layout =
            compute::make_pipeline_layout(&device, &[&compute_bind_group_layout]);
        let compute_pipeline =
            compute::make_pipeline(&device, &compute_pipeline_layout, &compute_shader);
        let compute_bind_group =
            compute::make_bind_group(&device, &compute_bind_group_layout, &buffers);

        let mut _self = Self {
            surface,
            device,
            queue,
            config,

            window,

            egui,

            srgb_format,
            unorm_format,

            render_bind_group_layout,
            render_pipeline,
            render_bind_group,

            compute_bind_group_layout,
            compute_pipeline,
            compute_bind_group,

            buffers,

            params,

            last_frame: [std::time::Instant::now(); constants::egui::TIME_INFO_LAST_N],
        };

        _self.resize_particles();
        _self.sync_uniform();

        Ok(_self)
    }

    pub fn resize_particles(&mut self) {
        // Resize buffers if needed
        if self.params.n > self.buffers.capacity {
            self.buffers.resize(&self.device, self.params.n);
            self.render_bind_group = renderer::make_bind_group(
                &self.device,
                &self.render_bind_group_layout,
                &self.buffers,
            );
            self.compute_bind_group = compute::make_bind_group(
                &self.device,
                &self.compute_bind_group_layout,
                &self.buffers,
            );
        }

        // Compute new initial positions and velocities
        let (pos, vel, col) = reset_galaxy(self.params.n);
        self.params.buffer_in_use = BufferInUse::Primary; // reset to primary on upload

        // Upload to GPU
        self.buffers.upload_data(
            &self.queue,
            Some(&pos),
            Some(&vel),
            Some(&col),
            Some(&self.params),
        );
    }

    pub fn sync_uniform(&mut self) {
        self.buffers
            .upload_data(&self.queue, None, None, None, Some(&self.params));
    }

    pub fn handle_egui_event(
        &mut self,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        if let Some(egui) = &mut self.egui {
            egui.handle_event(&self.window, event)
        } else {
            egui_winit::EventResponse::default()
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            // Ensure all operations are done before resizing
            _ = self.device.poll(wgpu::PollType::Wait);
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.sync_uniform(); // Ensure uniform is up to date
        let output = self.surface.get_current_texture()?;

        let srgb_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.srgb_format),
            ..Default::default()
        });

        let unorm_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.unorm_format),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // Update simulation state
        self._update(&mut encoder, None);
        // Render the scene
        self._render(&mut encoder, &srgb_view);
        // Render the egui UI
        self._render_egui(&mut encoder, &unorm_view);

        // Submit the commands
        self.queue.submit(Some(encoder.finish()));

        // Drop the views to release the borrow on the texture
        drop(unorm_view);
        drop(srgb_view);

        // Present the frame
        output.present();

        // Tick the buffer in use
        if !self.params.paused {
            self.params.buffer_in_use.tick();
        }

        Ok(())
    }

    /// Update simulation state via compute shader
    fn _update(&mut self, encoder: &mut wgpu::CommandEncoder, override_paused: Option<()>) {
        if self.params.paused && override_paused.is_none() {
            return;
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

        let workgroup_count = (self.params.n + constants::shader::WORKGROUP_SIZE - 1)
            / constants::shader::WORKGROUP_SIZE;

        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    fn _render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(constants::gpu::BACKGROUND_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.draw(0..self.params.n, 0..1);
    }

    fn _render_egui(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Render the egui frame
        if let Some(egui) = &mut self.egui {
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: egui.context.pixels_per_point(),
            };

            let mut params = std::mem::take(&mut self.params);
            let mut action = ParamsEguiAction::None;

            let mut last_frame = self.last_frame;

            egui.draw(
                &self.device,
                &self.queue,
                encoder,
                &self.window,
                view,
                &screen_descriptor,
                |ctx| {
                    egui::Window::new("Simulation Controls")
                        .default_width(300.0)
                        .resizable(true)
                        .show(ctx, |ui| {
                            action = params.render_info(ui, &mut last_frame);
                        });
                },
            );

            // Put the params back
            self.params = params;
            self.last_frame = last_frame;

            // Handle any actions from the UI
            match action {
                ParamsEguiAction::None => {}
                ParamsEguiAction::Reset
                | ParamsEguiAction::ParameterUpdated(ParticleUpdated::Less)
                | ParamsEguiAction::ParameterUpdated(ParticleUpdated::More) => {
                    self.resize_particles();
                }
                ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same) => {
                    self.sync_uniform();
                }
                ParamsEguiAction::Step => {
                    debug_assert!(
                        self.params.paused,
                        "Step action should only be possible when paused"
                    );
                    self._update(encoder, Some(()));
                    self.params.buffer_in_use.tick(); // Advance buffer even if paused
                }
            }
        }
    }
}
