//! Window list and control for the Shell UI (wlr-foreign-toplevel-management, version 3).
//!
//! The window switcher lists the open applications with their titles and application IDs,
//! raises one (activate), closes one (close), and the Home button minimises the application on
//! top so the home screen shows (set_minimized). In the compact layout every window fills the
//! window area, so requests to maximise, leave maximised or go full screen are ignored and the
//! state always reports maximised. Only the Shell UI may bind the global (clients.rs).

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols_wlr::foreign_toplevel::v1::server::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
        wayland_server::{
            backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
            Resource,
        },
    },
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    clients::{allowed, Privilege},
    state::UnitState,
};

const VERSION: u32 = 3;

/// What the switcher is told about a window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub activated: bool,
    pub minimized: bool,
}

impl ToplevelInfo {
    /// The `state` event's array: native-endian 32-bit values of the protocol's state enum.
    pub fn state_array(&self) -> Vec<u8> {
        let mut states = vec![zwlr_foreign_toplevel_handle_v1::State::Maximized];
        if self.minimized {
            states.push(zwlr_foreign_toplevel_handle_v1::State::Minimized);
        }
        if self.activated {
            states.push(zwlr_foreign_toplevel_handle_v1::State::Activated);
        }
        states
            .into_iter()
            .flat_map(|s| (s as u32).to_ne_bytes())
            .collect()
    }
}

struct Entry {
    id: u64,
    window: Window,
    sent: ToplevelInfo,
    handles: Vec<ZwlrForeignToplevelHandleV1>,
}

/// Bound managers and the windows they are told about.
pub struct ToplevelManagerState {
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    entries: Vec<Entry>,
    next_id: u64,
}

impl ToplevelManagerState {
    pub fn new(dh: &DisplayHandle) -> Self {
        dh.create_global::<UnitState, ZwlrForeignToplevelManagerV1, ()>(VERSION, ());
        Self {
            managers: Vec::new(),
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// A placeholder while the state is borrowed (see UnitState::refresh_toplevels).
    pub fn empty() -> Self {
        Self {
            managers: Vec::new(),
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn window(&self, id: u64) -> Option<Window> {
        self.entries.iter().find(|e| e.id == id).map(|e| e.window.clone())
    }

    /// A new application window: tell every bound manager.
    pub fn window_opened(&mut self, dh: &DisplayHandle, window: Window, info: ToplevelInfo) {
        let id = self.next_id;
        self.next_id += 1;
        let mut entry = Entry {
            id,
            window,
            sent: info,
            handles: Vec::new(),
        };
        for manager in &self.managers {
            announce(dh, manager, &mut entry);
        }
        self.entries.push(entry);
    }

    /// An application window was destroyed.
    pub fn window_closed(&mut self, window: &Window) {
        if let Some(pos) = self.entries.iter().position(|e| &e.window == window) {
            let entry = self.entries.remove(pos);
            for handle in entry.handles {
                handle.closed();
            }
        }
    }

    /// Sends what has changed (title, application ID, activated, minimised) since last time.
    pub fn refresh(&mut self, current: impl Fn(&Window) -> ToplevelInfo) {
        for entry in &mut self.entries {
            let now = current(&entry.window);
            if now == entry.sent {
                continue;
            }
            for handle in &entry.handles {
                if now.title != entry.sent.title {
                    handle.title(now.title.clone());
                }
                if now.app_id != entry.sent.app_id {
                    handle.app_id(now.app_id.clone());
                }
                if now.activated != entry.sent.activated || now.minimized != entry.sent.minimized {
                    handle.state(now.state_array());
                }
                handle.done();
            }
            entry.sent = now;
        }
    }

    fn handle_destroyed(&mut self, handle: &ZwlrForeignToplevelHandleV1) {
        for entry in &mut self.entries {
            entry.handles.retain(|h| h != handle);
        }
    }

    fn client_gone(&mut self, client: &ClientId) {
        self.managers.retain(|m| m.client().map(|c| &c.id() != client).unwrap_or(false));
        for entry in &mut self.entries {
            entry
                .handles
                .retain(|h| h.client().map(|c| &c.id() != client).unwrap_or(false));
        }
    }
}

/// Creates a handle for one window on one manager and sends what is known about it.
fn announce(dh: &DisplayHandle, manager: &ZwlrForeignToplevelManagerV1, entry: &mut Entry) {
    let Some(client) = manager.client() else { return };
    let Ok(handle) = client.create_resource::<ZwlrForeignToplevelHandleV1, u64, UnitState>(
        dh,
        manager.version(),
        entry.id,
    ) else {
        return;
    };
    manager.toplevel(&handle);
    handle.title(entry.sent.title.clone());
    handle.app_id(entry.sent.app_id.clone());
    handle.state(entry.sent.state_array());
    handle.done();
    entry.handles.push(handle);
}

/// Title and application ID of an application window.
pub fn title_and_app_id(window: &Window) -> (String, String) {
    window
        .toplevel()
        .map(|t| {
            with_states(t.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .map(|d| {
                        let d = d.lock().unwrap();
                        (d.title.clone().unwrap_or_default(), d.app_id.clone().unwrap_or_default())
                    })
                    .unwrap_or_default()
            })
        })
        .unwrap_or_default()
}

impl GlobalDispatch<ZwlrForeignToplevelManagerV1, ()> for UnitState {
    fn bind(
        state: &mut Self,
        dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let manager = data_init.init(resource, ());
        for entry in &mut state.toplevels.entries {
            announce(dh, &manager, entry);
        }
        state.toplevels.managers.push(manager);
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        allowed(&client, Privilege::ToplevelControl)
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for UnitState {
    fn request(
        state: &mut Self,
        _client: &Client,
        manager: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            state.toplevels.managers.retain(|m| m != manager);
            manager.finished();
        }
    }

    fn destroyed(state: &mut Self, client: ClientId, _manager: &ZwlrForeignToplevelManagerV1, _data: &()) {
        state.toplevels.client_gone(&client);
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, u64> for UnitState {
    fn request(
        state: &mut Self,
        _client: &Client,
        handle: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        id: &u64,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Request;
        let Some(window) = state.toplevels.window(*id) else { return };
        match request {
            Request::Activate { .. } | Request::UnsetMinimized => state.activate_window(&window),
            Request::SetMinimized => state.minimize_window(&window),
            Request::Close => {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_close();
                }
            }
            Request::Destroy => state.toplevels.handle_destroyed(handle),
            // Compact layout: windows always fill the window area.
            Request::SetMaximized
            | Request::UnsetMaximized
            | Request::SetFullscreen { .. }
            | Request::UnsetFullscreen
            | Request::SetRectangle { .. } => {}
            _ => {}
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, handle: &ZwlrForeignToplevelHandleV1, _id: &u64) {
        state.toplevels.handle_destroyed(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(info: &ToplevelInfo) -> Vec<u32> {
        info.state_array()
            .chunks(4)
            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn always_reports_maximised() {
        assert_eq!(values(&ToplevelInfo::default()), vec![0]);
    }

    #[test]
    fn reports_activated_and_minimised() {
        let active = ToplevelInfo { activated: true, ..Default::default() };
        assert_eq!(values(&active), vec![0, 2]);
        let minimised = ToplevelInfo { minimized: true, ..Default::default() };
        assert_eq!(values(&minimised), vec![0, 1]);
    }
}
