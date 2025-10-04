mod buffers;
mod compute;
mod egui_renderer;
mod renderer;

use bytemuck::cast_slice;
pub use egui_renderer::EguiRenderer;

use std::sync::Arc;

use winit::window::Window;

use crate::{
    constants,
    gpu::{
        buffers::GpuBuffers,
        renderer::{
            make_render_bg, make_render_bgl, make_render_pipeline, make_render_pipeline_layout,
            make_render_shader,
        },
    },
    sim::{self, ParamsEguiAction, ParticleUpdated, reset_galaxy},
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
    buffers: buffers::GpuBuffers,

    // Simulation state
    params: sim::SimParams,
}

fn setup_params() -> sim::SimParams {
    sim::SimParams {
        dt: 0.016, // ~60 FPS
        g: -9.81,
        damping: constants::sim::DAMPING,
        softening: 0.1,
        n: constants::sim::INITIAL_PARTICLES,
        world: constants::sim::WORLD_SIZE,
        wrap: constants::sim::WRAP,
    }
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: srgb_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
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

        let shader = make_render_shader(&device);
        let render_bind_group_layout = make_render_bgl(&device);
        let render_pipeline_layout =
            make_render_pipeline_layout(&device, &[&render_bind_group_layout]);
        let render_pipeline =
            make_render_pipeline(&device, &render_pipeline_layout, &shader, srgb_format);

        let params = setup_params();
        let buffers = GpuBuffers::create(&device, params.n);

        let render_bind_group = make_render_bg(&device, &render_bind_group_layout, &buffers);

        // Populate initial particle data (to be moved to sim module later)
        let (pos, vel, col) = reset_galaxy(params.n);
        queue.write_buffer(&buffers.positions, 0, cast_slice(&pos));
        queue.write_buffer(&buffers.velocities, 0, cast_slice(&vel));
        queue.write_buffer(&buffers.colors, 0, cast_slice(&col));

        let uni = params.to_uniform();
        queue.write_buffer(&buffers.uniform, 0, cast_slice(std::slice::from_ref(&uni)));

        Ok(Self {
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
            buffers,

            params,
        })
    }

    pub fn resize_particles(&mut self) {
        if self.params.n > self.buffers.capacity {
            log::info!(
                "Resizing particle buffers: {} -> {}",
                self.buffers.capacity,
                self.params.n
            );
            self.buffers = GpuBuffers::create(&self.device, self.params.n);
            self.render_bind_group =
                make_render_bg(&self.device, &self.render_bind_group_layout, &self.buffers);
        }

        let (pos, vel, col) = reset_galaxy(self.params.n);
        self.queue
            .write_buffer(&self.buffers.positions, 0, cast_slice(&pos));
        self.queue
            .write_buffer(&self.buffers.velocities, 0, cast_slice(&vel));
        self.queue
            .write_buffer(&self.buffers.colors, 0, cast_slice(&col));
    }

    pub fn sync_uniform(&mut self) {
        let uni = self.params.to_uniform();
        self.queue.write_buffer(
            &self.buffers.uniform,
            0,
            cast_slice(std::slice::from_ref(&uni)),
        );
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
        // Currently just clears the screen with a solid color
        let output = self.surface.get_current_texture()?;

        let srgb_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.srgb_format),
            ..Default::default()
        });

        let unorm_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.unorm_format),
            ..Default::default()
        });

        // Clear the screen with a solid color
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &srgb_view,
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

        // Render the egui frame
        if let Some(egui) = &mut self.egui {
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: egui.context.pixels_per_point(),
            };

            let mut params = std::mem::take(&mut self.params);
            let mut action = ParamsEguiAction::None;

            egui.draw(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.window,
                &unorm_view,
                &screen_descriptor,
                |ctx| {
                    egui::Window::new("Simulation Controls")
                        .default_width(300.0)
                        .resizable(true)
                        .show(ctx, |ui| {
                            action = params.render_info(ui);
                        });
                },
            );

            // Put the params back
            self.params = params;

            // Handle any actions from the UI
            match action {
                ParamsEguiAction::None => {}
                ParamsEguiAction::Reset
                | ParamsEguiAction::ParameterUpdated(ParticleUpdated::Less)
                | ParamsEguiAction::ParameterUpdated(ParticleUpdated::More) => {
                    self.resize_particles();
                    self.sync_uniform();
                }
                ParamsEguiAction::ParameterUpdated(ParticleUpdated::Same) => {
                    self.sync_uniform();
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        drop(unorm_view);
        drop(srgb_view);

        output.present(); // Present the frame

        Ok(())
    }
}
