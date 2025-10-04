use crate::constants;

pub struct EguiRenderer {
    pub context: egui::Context,
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        output_depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
        dithering: bool,
        window: &winit::window::Window,
    ) -> Self {
        let context = egui::Context::default();
        let viewport_id = context.viewport_id();

        let visuals = egui::Visuals {
            window_corner_radius: constants::egui::BORDER_RADIUS,
            window_shadow: constants::egui::SHADOW,
            ..Default::default()
        };

        context.set_visuals(visuals); // Apply custom visuals to context

        let state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            &window,
            None, // native_pixels_per_point
            None, // theme
            None, // max_texture_side
        );

        let renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            dithering,
        );

        Self {
            context,
            renderer,
            state,
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        run_ui: impl FnMut(&egui::Context),
    ) {
        let raw_input = self.state.take_egui_input(window); // Get input from the window
        let full_output = self.context.run(raw_input, run_ui); // Run the egui pipeline

        // Now handle the output
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, id, &image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &tris, screen_descriptor);

        let mut rpass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: window_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                label: Some("Egui main render pass"),
                timestamp_writes: None,
                depth_stencil_attachment: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        self.renderer.render(&mut rpass, &tris, screen_descriptor);
        drop(rpass); // End the render pass

        for id in full_output.textures_delta.free {
            self.renderer.free_texture(&id);
        }
    }
}
