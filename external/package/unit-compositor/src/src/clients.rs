//! Client roles and the programs the compositor starts.
//!
//! Some Wayland protocols give a client power over the whole screen or over other applications:
//! layer surfaces draw above every window, toplevel control lists, raises and closes other
//! applications' windows, and the input method sees every key typed into a text field. Only the
//! programs the compositor starts itself get them, each on its own connection that carries its
//! role, so no application that connects through the Wayland socket can claim them:
//!
//! - the Shell UI (`--shell`): layer surfaces, toplevel control and the virtual keyboard (for the
//!   on-screen keyboard);
//! - the input method, Fcitx5 (`--input-method`): input-method and virtual keyboard.
//!
//! Applications use only the ordinary protocols (xdg-shell windows, text-input for typing).
//! The compositor restarts the Shell UI and the input method if they exit, with a limit so a
//! program that fails at start-up does not restart in a tight loop.

use std::{
    os::unix::{io::AsRawFd, net::UnixStream, process::CommandExt},
    process::{Child, Command},
    sync::Arc,
    time::{Duration, Instant},
};

use smithay::reexports::{
    calloop::{
        timer::{TimeoutAction, Timer},
        LoopHandle,
    },
    rustix,
    wayland_server::Client,
};

use crate::state::{ClientState, UnitState};

/// What a client is allowed to do beyond the ordinary protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientRole {
    /// An application, connected through the Wayland socket.
    #[default]
    App,
    /// The Shell UI, started by the compositor.
    Shell,
    /// The input method (Fcitx5), started by the compositor.
    InputMethod,
}

/// Protocols that only some roles may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    /// wlr-layer-shell: surfaces above or below every window (status bar, home, quick settings).
    LayerShell,
    /// wlr-foreign-toplevel-management: list, raise, minimise and close application windows.
    ToplevelControl,
    /// input-method v2: receive the text typed into the focused text field.
    InputMethod,
    /// virtual-keyboard v1: send key presses as if typed.
    VirtualKeyboard,
}

impl ClientRole {
    pub fn may_use(self, privilege: Privilege) -> bool {
        match privilege {
            Privilege::LayerShell | Privilege::ToplevelControl => self == ClientRole::Shell,
            Privilege::InputMethod => self == ClientRole::InputMethod,
            Privilege::VirtualKeyboard => {
                matches!(self, ClientRole::Shell | ClientRole::InputMethod)
            }
        }
    }

    fn name(self) -> &'static str {
        match self {
            ClientRole::App => "application",
            ClientRole::Shell => "shell",
            ClientRole::InputMethod => "input method",
        }
    }
}

/// The role of a connected client (applications by default).
pub fn role_of(client: &Client) -> ClientRole {
    client
        .get_data::<ClientState>()
        .map(|data| data.role)
        .unwrap_or_default()
}

/// Whether a client may use a privileged protocol: used as the filter of its global, so other
/// clients never see the global at all.
pub fn allowed(client: &Client, privilege: Privilege) -> bool {
    role_of(client).may_use(privilege)
}

/// Programs to start, from the command line:
///
/// ```text
/// unit-compositor [--shell <program>] [--input-method <program>] [--command <program> [args...]]
/// ```
///
/// `--command` starts an ordinary client (for development and test boots) and takes the rest of
/// the command line as its arguments, so it comes last.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Launch {
    pub shell: Option<String>,
    pub input_method: Option<String>,
    pub command: Option<Vec<String>>,
}

impl Launch {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut launch = Launch::default();
        let mut i = 0;
        while i < args.len() {
            let value = |i: usize| {
                args.get(i + 1)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| format!("{} needs a program", args[i]))
            };
            match args[i].as_str() {
                "--shell" => {
                    launch.shell = Some(value(i)?);
                    i += 2;
                }
                "--input-method" => {
                    launch.input_method = Some(value(i)?);
                    i += 2;
                }
                "--command" => {
                    value(i)?;
                    launch.command = Some(args[i + 1..].to_vec());
                    break;
                }
                other => return Err(format!("unknown option {}", other)),
            }
        }
        Ok(launch)
    }
}

/// Restarts allowed within `RESTART_WINDOW` before the compositor gives up on a program.
const RESTART_LIMIT: usize = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const RESTART_DELAY: Duration = Duration::from_secs(1);
const CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Whether a program that exited may be restarted, given the times of its earlier starts.
/// Keeps only the starts within the window.
pub fn may_restart(starts: &mut Vec<Instant>, now: Instant) -> bool {
    starts.retain(|t| now.duration_since(*t) < RESTART_WINDOW);
    starts.len() < RESTART_LIMIT
}

/// A program started by the compositor with a privileged connection, restarted if it exits.
struct Supervised {
    program: String,
    role: ClientRole,
    child: Option<Child>,
    starts: Vec<Instant>,
    /// Exited, and to be started again at the next check.
    restart_due: bool,
}

/// Starts `program` with its own connection to the compositor that carries `role`, and checks
/// on it until the compositor exits.
pub fn supervise(handle: &LoopHandle<'static, UnitState>, state: &mut UnitState, program: String, role: ClientRole) {
    let mut supervised = Supervised {
        program,
        role,
        child: None,
        starts: Vec::new(),
        restart_due: false,
    };
    supervised.start(state);
    let _ = handle.insert_source(Timer::from_duration(CHECK_INTERVAL), move |_, _, state| {
        match supervised.check(state) {
            Some(delay) => TimeoutAction::ToDuration(delay),
            None => TimeoutAction::Drop,
        }
    });
}

impl Supervised {
    fn start(&mut self, state: &mut UnitState) {
        self.starts.push(Instant::now());
        match spawn_with_connection(state, &self.program, self.role) {
            Ok(child) => {
                tracing::info!("started the {} ({})", self.role.name(), self.program);
                self.child = Some(child);
            }
            Err(err) => {
                tracing::error!("could not start the {} ({}): {}", self.role.name(), self.program, err);
                self.child = None;
            }
        }
    }

    /// Returns when to check again, or None to stop.
    fn check(&mut self, state: &mut UnitState) -> Option<Duration> {
        if self.restart_due {
            self.restart_due = false;
            self.start(state);
            return Some(CHECK_INTERVAL);
        }
        let running = match self.child.as_mut().map(|c| c.try_wait()) {
            Some(Ok(None)) => true,
            Some(Ok(Some(status))) => {
                tracing::warn!("the {} exited ({})", self.role.name(), status);
                false
            }
            Some(Err(err)) => {
                tracing::warn!("the {} could not be checked: {}", self.role.name(), err);
                false
            }
            None => false,
        };
        if running {
            return Some(CHECK_INTERVAL);
        }
        self.child = None;
        if !may_restart(&mut self.starts, Instant::now()) {
            tracing::error!(
                "the {} stopped {} times within {} s: not restarting it",
                self.role.name(),
                RESTART_LIMIT,
                RESTART_WINDOW.as_secs()
            );
            return None;
        }
        self.restart_due = true;
        Some(RESTART_DELAY)
    }
}

/// Starts a program connected to the compositor through one end of a socket pair, passed as
/// WAYLAND_SOCKET, with the other end registered as a client with the given role. The Wayland
/// library uses WAYLAND_SOCKET for the program's first connection and then clears it; programs
/// the program starts itself connect through WAYLAND_DISPLAY as applications.
fn spawn_with_connection(state: &mut UnitState, program: &str, role: ClientRole) -> std::io::Result<Child> {
    let (ours, theirs) = UnixStream::pair()?;
    let fd = theirs.as_raw_fd();
    let mut command = Command::new(program);
    command
        .env("WAYLAND_DISPLAY", &state.socket_name)
        .env("WAYLAND_SOCKET", fd.to_string());
    // Safety: only an fcntl system call runs between fork and exec. It keeps the program's end
    // of the socket open across exec; every other descriptor keeps its close-on-exec flag.
    unsafe {
        command.pre_exec(move || {
            let fd = rustix::fd::BorrowedFd::borrow_raw(fd);
            rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::empty()).map_err(std::io::Error::from)
        });
    }
    let child = command.spawn()?;
    drop(theirs);
    state
        .display_handle
        .insert_client(
            ours,
            Arc::new(ClientState {
                role,
                ..Default::default()
            }),
        )
        .map_err(std::io::Error::other)?;
    Ok(child)
}

/// Starts an ordinary client through the Wayland socket (`--command`).
pub fn spawn_app(state: &UnitState, args: &[String]) {
    let mut command = Command::new(&args[0]);
    command.args(&args[1..]).env("WAYLAND_DISPLAY", &state.socket_name);
    if let Err(err) = command.spawn() {
        tracing::warn!("could not start {}: {}", args[0], err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn only_the_shell_gets_layer_surfaces_and_toplevel_control() {
        for p in [Privilege::LayerShell, Privilege::ToplevelControl] {
            assert!(ClientRole::Shell.may_use(p));
            assert!(!ClientRole::App.may_use(p));
            assert!(!ClientRole::InputMethod.may_use(p));
        }
    }

    #[test]
    fn only_the_input_method_reads_typed_text() {
        assert!(ClientRole::InputMethod.may_use(Privilege::InputMethod));
        assert!(!ClientRole::Shell.may_use(Privilege::InputMethod));
        assert!(!ClientRole::App.may_use(Privilege::InputMethod));
    }

    #[test]
    fn applications_cannot_send_virtual_keys() {
        assert!(!ClientRole::App.may_use(Privilege::VirtualKeyboard));
        assert!(ClientRole::Shell.may_use(Privilege::VirtualKeyboard));
        assert!(ClientRole::InputMethod.may_use(Privilege::VirtualKeyboard));
    }

    #[test]
    fn parses_programs() {
        assert_eq!(Launch::parse(&[]).unwrap(), Launch::default());
        let l = Launch::parse(&args("--shell /usr/bin/s --input-method /usr/bin/fcitx5 --command a -x b")).unwrap();
        assert_eq!(l.shell.as_deref(), Some("/usr/bin/s"));
        assert_eq!(l.input_method.as_deref(), Some("/usr/bin/fcitx5"));
        assert_eq!(l.command, Some(args("a -x b")));
    }

    #[test]
    fn command_takes_the_rest_of_the_line() {
        let l = Launch::parse(&args("--command a --shell b")).unwrap();
        assert_eq!(l.command, Some(args("a --shell b")));
        assert_eq!(l.shell, None);
    }

    #[test]
    fn rejects_missing_programs_and_unknown_options() {
        assert!(Launch::parse(&args("--shell")).is_err());
        assert!(Launch::parse(&args("--shell --command a")).is_err());
        assert!(Launch::parse(&args("--command")).is_err());
        assert!(Launch::parse(&args("--other x")).is_err());
    }

    #[test]
    fn restarts_are_limited_within_a_minute() {
        let t0 = Instant::now();
        let mut starts = vec![t0; RESTART_LIMIT - 1];
        assert!(may_restart(&mut starts, t0));
        starts.push(t0);
        assert!(!may_restart(&mut starts, t0 + Duration::from_secs(1)));
        // A minute later the old starts no longer count.
        assert!(may_restart(&mut starts, t0 + RESTART_WINDOW));
        assert!(starts.is_empty());
    }
}
