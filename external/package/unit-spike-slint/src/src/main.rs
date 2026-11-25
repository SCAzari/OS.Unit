//! Toolkit spike (UNIT-OS-WP-0010): Slint version of the spike screens.
//!   unit-spike-slint sign-in | settings | display

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let screen = std::env::args().nth(1).unwrap_or_else(|| "sign-in".into());
    let window = SpikeWindow::new()?;
    window.set_page(match screen.as_str() {
        "settings" | "display" => screen.as_str().into(),
        _ => "sign-in".into(),
    });
    window.run()
}
