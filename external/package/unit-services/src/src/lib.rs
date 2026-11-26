//! Unit system services for the Shell UI, first slice (UNIT-OS-WP-0009).
//!
//! Software Architecture r1.16 §4 has the Shell UI as a view over Unit services. This package
//! holds the smallest versions of three of them, each its own program and systemd user service
//! on the session bus:
//!
//! - the Settings service (`unit-settings`): typed settings with limits, kept per profile across
//!   restarts (the Settings interface, §11.1);
//! - the Notification service (`unit-notifications`): the freedesktop notification interface for
//!   applications, and the notification centre, banners and Do not disturb for the Shell UI;
//! - the Session service (`unit-session`): the first slice of the Session and mode manager. It
//!   lists and launches applications from their desktop entries and asks an application to save
//!   before it is closed (the save-and-resume contract, §6.7, in a minimal form). The compositor,
//!   not this service, starts the Shell UI and restarts it if it fails (Executive decision,
//!   26 Nov 2026).
//!
//! Each Unit D-Bus interface has a versioned definition file in `../interfaces`; a test checks
//! that the service still matches it.

pub mod desktop;
pub mod notifications;
pub mod session;
pub mod settings;
pub mod store;

#[cfg(test)]
mod testdir;

/// Logs to standard error (the journal, under systemd); `RUST_LOG` sets the level.
pub fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .init();
}

/// Checks a service's introspection against its interface definition file. With
/// `UNIT_UPDATE_INTERFACES=1` it rewrites the file instead, for a deliberate interface change
/// (which also raises the interface's version).
#[cfg(test)]
pub(crate) fn check_interface_file<I: zbus::object_server::Interface>(service: &I, file: &str) {
    let mut xml = String::new();
    service.introspect_to_writer(&mut xml, 0);
    let expected = format!(
        "<!-- {} D-Bus interface (UNIT-OS-WP-0009). Checked by the services' unit tests. -->\n{}",
        I::name(),
        xml
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../interfaces").join(file);
    if std::env::var_os("UNIT_UPDATE_INTERFACES").is_some() {
        std::fs::write(&path, &expected).unwrap();
        return;
    }
    let actual = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    assert_eq!(
        actual, expected,
        "{} no longer matches the service: change the interface version if this is intended",
        file
    );
}
