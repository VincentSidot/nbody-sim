mod app;
mod constants;
mod gpu;
mod sim;
mod utils;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;

fn main() {
    utils::logger::init_logger();

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll); // Continuously poll for events

    let mut app = App::default();

    if let Err(err) = event_loop.run_app(&mut app) {
        log::error!("Application exited with event loop error: {err}");
    }
}
