//! Unit compositor, first version (work package UNIT-OS-WP-0009, Desktop Foundation - Increment 1).
//!
//! Wayland compositor and window manager of Unit OS, built on the Smithay library as Software
//! Architecture r1.16 §4 and §7 describe. This first version drives one display through DRM/KMS,
//! renders with OpenGL ES, takes keyboard, pointer and touch input through libinput, and shows
//! applications in the compact (phone) layout: one application window fills the display at a time.

mod clients;
mod handlers;
mod ime;
mod input;
mod layer;
mod layout;
mod state;
mod toplevel;
mod udev;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::UnitState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_env("UNIT_COMPOSITOR_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).init();

    let mut event_loop: EventLoop<UnitState> = EventLoop::try_new()?;
    let display: Display<UnitState> = Display::new()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let launch = clients::Launch::parse(&args)?;
    let mut state = UnitState::new(&mut event_loop, display);

    udev::init(&mut event_loop, &mut state)?;

    // The Shell UI and the input method, each with a connection that carries its role, restarted
    // if they exit; an ordinary first client for development and test boots (clients.rs).
    let handle = event_loop.handle();
    if let Some(program) = launch.shell {
        clients::supervise(&handle, &mut state, program, clients::ClientRole::Shell);
    }
    if let Some(program) = launch.input_method {
        clients::supervise(&handle, &mut state, program, clients::ClientRole::InputMethod);
    }
    if let Some(command) = launch.command {
        clients::spawn_app(&state, &command);
    }

    tracing::info!(
        "unit-compositor {} running on WAYLAND_DISPLAY={}",
        env!("CARGO_PKG_VERSION"),
        state.socket_name.to_string_lossy()
    );
    event_loop.run(None, &mut state, |state| {
        state.space.refresh();
        state.popups.cleanup();
        for output in state.space.outputs() {
            smithay::desktop::layer_map_for_output(output).cleanup();
        }
        state.refresh_toplevels();
        let _ = state.display_handle.flush_clients();
    })?;
    Ok(())
}
