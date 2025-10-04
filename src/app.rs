use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

use crate::{constants, gpu};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    state: Option<gpu::State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(constants::window::TITLE))
                .expect("Failed to create window"),
        );

        let state = pollster::block_on(gpu::State::new(window.clone(), true))
            .expect("Failed to create GPU state");

        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = match &self.window {
            Some(window) if window.id() == window_id => window,
            _ => {
                log::warn!("Received event for unknown window: {:?}", window_id);
                return;
            }
        };
        let state = match &mut self.state {
            Some(state) => state,
            None => {
                log::warn!("Received window event but GPU state is not initialized");
                return;
            }
        };

        // Handle egui events
        let response = state.handle_egui_event(&event);
        if response.repaint {
            window.request_redraw();
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested, terminating application");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                // Handle redraw
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of memory, terminating application");
                        event_loop.exit();
                    }
                    Err(e) => log::error!("Failed to render frame: {:?}", e),
                }
            }

            WindowEvent::Resized(new_size) => {
                state.resize(new_size.width, new_size.height);
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                // I think this is not correct, but I need to to something here
                let size = window.inner_size();
                state.resize(size.width, size.height);
            }

            _ => {}
        }
    }
}
