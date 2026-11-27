//! The Session service: the first slice of the Session and mode manager (Software Architecture
//! r1.16 §4).
//!
//! For the Shell UI it lists the installed applications, starts one when the user picks it in the
//! launcher or the dock, and, before the user closes an application, asks it to save (the
//! save-and-resume contract, §6.7, in a minimal form: only the Save state message).
//!
//! An application that takes part registers through the lifecycle interface, giving its
//! application ID, and serves `os.unit.LifecycleClient1.SaveState` on its own connection. When the
//! user closes it, the Session service asks it to save and waits up to the deadline; the Shell UI
//! then closes the window. An application that has not registered is closed with notice, as §6.7
//! sets out: the Shell UI tells the user its unsaved work may not have been kept.
//!
//! Only applications with a desktop entry can be started, never an arbitrary command. The
//! compositor starts the Shell UI itself (Executive decision, 26 Nov 2026), so this service does
//! not. Modes, docking, graphics handover, Suspend and Restore come in stage 3.

use std::{
    collections::HashMap,
    os::unix::process::{CommandExt, ExitStatusExt},
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use zbus::{interface, message::Header, object_server::SignalEmitter, Connection};

use crate::desktop::{self, App};

pub const BUS_NAME: &str = "os.unit.Session1";
pub const OBJECT_PATH: &str = "/os/unit/Session1";
pub const LIFECYCLE_PATH: &str = "/os/unit/Lifecycle1";
/// Where a registered application serves the client side of the lifecycle interface.
pub const CLIENT_PATH: &str = "/os/unit/LifecycleClient1";
pub const CLIENT_INTERFACE: &str = "os.unit.LifecycleClient1";

/// How long an application has to save before it is closed. A working figure until the bounded
/// times are set under Design decision D-06 (performance and power targets).
pub const SAVE_DEADLINE: Duration = Duration::from_secs(5);

/// The result of asking an application to save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    /// It saved and returned a state token.
    Saved,
    /// It has not registered: it is closed with notice.
    NotRegistered,
    /// It did not answer before the deadline.
    NoResponse,
    /// It answered with an error, or could not be reached.
    Failed,
}

impl CloseOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            CloseOutcome::Saved => "saved",
            CloseOutcome::NotRegistered => "not-registered",
            CloseOutcome::NoResponse => "no-response",
            CloseOutcome::Failed => "failed",
        }
    }
}

/// Turns the answer to Save state (None if the deadline passed) into an outcome and a token.
pub fn close_outcome(answer: Option<Result<String, String>>) -> (CloseOutcome, String) {
    match answer {
        Some(Ok(token)) => (CloseOutcome::Saved, token),
        Some(Err(e)) => {
            tracing::warn!("save before closing failed: {}", e);
            (CloseOutcome::Failed, String::new())
        }
        None => (CloseOutcome::NoResponse, String::new()),
    }
}

/// Applications registered for the lifecycle interface: application ID to bus connection name.
#[derive(Debug, Default)]
pub struct Registry(HashMap<String, String>);

impl Registry {
    pub fn register(&mut self, app_id: &str, sender: &str) {
        self.0.insert(app_id.to_string(), sender.to_string());
    }

    /// Removes the registration only if it is still the sender's.
    pub fn unregister(&mut self, app_id: &str, sender: &str) -> bool {
        if self.0.get(app_id).map(String::as_str) == Some(sender) {
            self.0.remove(app_id);
            true
        } else {
            false
        }
    }

    pub fn connection_of(&self, app_id: &str) -> Option<String> {
        self.0.get(app_id).cloned()
    }
}

pub type SharedRegistry = Arc<Mutex<Registry>>;

/// The Session interface, version 1, for the Shell UI.
pub struct SessionService {
    pub dirs: Vec<PathBuf>,
    pub registry: SharedRegistry,
}

impl SessionService {
    fn find(&self, app_id: &str) -> Option<App> {
        desktop::scan(&self.dirs).into_iter().find(|a| a.id == app_id)
    }
}

/// D-Bus errors of the Session interface.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "os.unit.Session1.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    UnknownApp(String),
    LaunchFailed(String),
}

#[interface(name = "os.unit.Session1")]
impl SessionService {
    /// The installed applications, by name: ID, name, icon name and comment.
    async fn list_apps(&self) -> Vec<(String, String, String, String)> {
        desktop::scan(&self.dirs)
            .into_iter()
            .map(|a| (a.id, a.name, a.icon, a.comment))
            .collect()
    }

    /// Starts an application by its ID. Returns its process ID.
    async fn launch(&self, app_id: &str, #[zbus(connection)] conn: &Connection) -> Result<u32, Error> {
        let app = self.find(app_id).ok_or_else(|| Error::UnknownApp(app_id.to_string()))?;
        let mut command = Command::new(&app.exec[0]);
        // Its own process group, so it is not stopped with this service.
        command.args(&app.exec[1..]).process_group(0);
        let mut child = command
            .spawn()
            .map_err(|e| Error::LaunchFailed(format!("{}: {}", app.exec[0], e)))?;
        let pid = child.id();
        tracing::info!("started {} ({})", app.id, pid);
        // Wait for it in the background, so it does not linger as a zombie, and tell the Shell UI.
        let conn = conn.clone();
        let id = app.id.clone();
        std::thread::spawn(move || {
            let code = match child.wait() {
                Ok(status) => status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
                Err(_) => -1,
            };
            tracing::info!("{} ({}) exited with {}", id, pid, code);
            let _ = zbus::block_on(async {
                let emitter = SignalEmitter::new(&conn, OBJECT_PATH)?;
                SessionService::app_exited(&emitter, &id, pid, code).await
            });
        });
        Ok(pid)
    }

    /// Asks an application to save before the user closes it, and waits up to the deadline.
    /// Returns the outcome ("saved", "not-registered", "no-response" or "failed") and the state
    /// token it returned. The Shell UI then closes the window.
    async fn prepare_close(&self, app_id: &str, #[zbus(connection)] conn: &Connection) -> (String, String) {
        let Some(dest) = self.registry.lock().unwrap().connection_of(app_id) else {
            return (CloseOutcome::NotRegistered.as_str().to_string(), String::new());
        };
        let deadline_ms = SAVE_DEADLINE.as_millis() as u32;
        let call = async {
            let reply = conn
                .call_method(Some(dest.as_str()), CLIENT_PATH, Some(CLIENT_INTERFACE), "SaveState", &(deadline_ms,))
                .await
                .map_err(|e| e.to_string())?;
            reply.body().deserialize::<String>().map_err(|e| e.to_string())
        };
        let answer = futures_lite::future::or(async { Some(call.await) }, async {
            async_io::Timer::after(SAVE_DEADLINE).await;
            None
        })
        .await;
        let (outcome, token) = close_outcome(answer);
        tracing::info!("{} before closing: {}", app_id, outcome.as_str());
        (outcome.as_str().to_string(), token)
    }

    /// The interface version.
    #[zbus(property)]
    fn version(&self) -> u32 {
        1
    }

    /// An application started by this service exited, with its exit code (128 plus the signal
    /// number if a signal stopped it).
    #[zbus(signal)]
    async fn app_exited(emitter: &SignalEmitter<'_>, app_id: &str, pid: u32, code: i32) -> zbus::Result<()>;
}

/// The lifecycle interface, version 1, for applications.
pub struct Lifecycle {
    pub dirs: Vec<PathBuf>,
    pub registry: SharedRegistry,
}

#[interface(name = "os.unit.Lifecycle1")]
impl Lifecycle {
    /// Takes part in the lifecycle: the caller will be asked to save (SaveState on
    /// `/os/unit/LifecycleClient1`) before it is closed. The ID must be an installed application.
    async fn register(&self, app_id: &str, #[zbus(header)] header: Header<'_>) -> Result<(), Error> {
        if !desktop::scan(&self.dirs).iter().any(|a| a.id == app_id) {
            return Err(Error::UnknownApp(app_id.to_string()));
        }
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        self.registry.lock().unwrap().register(app_id, &sender);
        tracing::info!("{} registered for the lifecycle ({})", app_id, sender);
        Ok(())
    }

    async fn unregister(&self, app_id: &str, #[zbus(header)] header: Header<'_>) {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        self.registry.lock().unwrap().unregister(app_id, &sender);
    }

    /// The interface version.
    #[zbus(property)]
    fn version(&self) -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcomes_of_save_before_closing() {
        assert_eq!(close_outcome(Some(Ok("t1".into()))), (CloseOutcome::Saved, "t1".to_string()));
        assert_eq!(close_outcome(Some(Err("gone".into()))).0, CloseOutcome::Failed);
        assert_eq!(close_outcome(None).0, CloseOutcome::NoResponse);
    }

    #[test]
    fn only_the_registered_connection_can_unregister() {
        let mut r = Registry::default();
        r.register("editor", ":1.5");
        assert!(!r.unregister("editor", ":1.9"));
        assert_eq!(r.connection_of("editor").as_deref(), Some(":1.5"));
        // Registering again (a restarted application) replaces the connection.
        r.register("editor", ":1.7");
        assert!(r.unregister("editor", ":1.7"));
        assert_eq!(r.connection_of("editor"), None);
    }

    /// The client side, which a registered application serves; defined here so that its
    /// interface file is checked like the others.
    struct ClientDefinition;

    #[interface(name = "os.unit.LifecycleClient1")]
    impl ClientDefinition {
        /// Save now, within `deadline_ms` milliseconds; return a state token that Restore will
        /// give back (empty if there is nothing to restore).
        async fn save_state(&self, _deadline_ms: u32) -> String {
            String::new()
        }
    }

    #[test]
    fn matches_its_interface_files() {
        crate::check_interface_file(&ClientDefinition, "os.unit.LifecycleClient1.xml");
        let registry = SharedRegistry::default();
        crate::check_interface_file(
            &SessionService { dirs: vec![], registry: registry.clone() },
            "os.unit.Session1.xml",
        );
        crate::check_interface_file(&Lifecycle { dirs: vec![], registry }, "os.unit.Lifecycle1.xml");
    }
}
