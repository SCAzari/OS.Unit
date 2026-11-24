//! Unit compositor, first version (work package UNIT-OS-WP-0009, Desktop Foundation - Increment 1).
//!
//! Wayland compositor and window manager of Unit OS, built on the Smithay library as Software
//! Architecture r1.16 §4 and §7 describe. This first version drives one display through DRM/KMS,
//! renders with OpenGL ES, takes keyboard, pointer and touch input through libinput, and shows
//! applications in the compact (phone) layout: one application window fills the display at a time.

mod handlers;
mod ime;
mod input;
mod layout;
mod state;
mod timing;
mod udev;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::UnitState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_env("UNIT_COMPOSITOR_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).init();

    let mut event_loop: EventLoop<UnitState> = EventLoop::try_new()?;
    let display: Display<UnitState> = Display::new()?;
    let mut state = UnitState::new(&mut event_loop, display);

    udev::init(&mut event_loop, &mut state)?;

    // Optional first client, for development and the demonstration build:
    //   unit-compositor --command <program> [args...]
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--command") && args.len() > 1 {
        let mut cmd = std::process::Command::new(&args[1]);
        cmd.args(&args[2..]).env("WAYLAND_DISPLAY", &state.socket_name);
        if let Err(err) = cmd.spawn() {
            tracing::warn!("could not start {}: {}", args[1], err);
        }
    }

    tracing::info!(
        "unit-compositor {} running on WAYLAND_DISPLAY={}",
        env!("CARGO_PKG_VERSION"),
        state.socket_name.to_string_lossy()
    );
    event_loop.run(None, &mut state, |state| {
        state.space.refresh();
        state.popups.cleanup();
        let _ = state.display_handle.flush_clients();
    })?;
    Ok(())
}
