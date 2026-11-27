//! Wayland protocol handlers: compositor, shared memory, seat, data device, output and xdg-shell,
//! and window management.
//!
//! Window management follows the compact layout of the Interface Design: each application
//! window fills the window area (the display less the zones the Shell UI's layer surfaces keep,
//! such as the status bar) and is raised with keyboard focus. The Shell UI can minimise the
//! window on top (Home) and activate any window (the window switcher), through toplevel.rs.

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::{find_popup_root_surface, get_popup_toplevel_coords, PopupKind, Window},
    input::{Seat, SeatHandler, SeatState},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_buffer, wl_seat, wl_surface::WlSurface},
            Client, Resource,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        output::OutputHandler,
        selection::{
            data_device::{
                set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
                ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::state::{ClientState, UnitState};

impl CompositorHandler for UnitState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.window_for_surface(&root) {
                window.on_commit();
            }
        }

        // First commit of a toplevel: send the initial configure.
        if let Some(window) = self.window_for_surface(surface) {
            let initial_configure_sent = with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            if !initial_configure_sent {
                window.toplevel().unwrap().send_configure();
            }
        }

        self.layer_commit(surface);

        self.popups.commit(surface);
        match self.popups.find_popup(surface) {
            Some(PopupKind::Xdg(ref popup)) if !popup.is_initial_configure_sent() => {
                popup.send_configure().expect("initial popup configure");
            }
            Some(PopupKind::InputMethod(ref popup)) => crate::ime::place_popup(popup),
            _ => {}
        }
    }
}

impl BufferHandler for UnitState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for UnitState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for UnitState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<UnitState> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {}

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
        // Only the window with keyboard focus is shown as active.
        for window in self.space.elements() {
            if let Some(toplevel) = window.toplevel() {
                let active = Some(toplevel.wl_surface()) == focused;
                toplevel.with_pending_state(|state| {
                    if active {
                        state.states.set(xdg_toplevel::State::Activated);
                    } else {
                        state.states.unset(xdg_toplevel::State::Activated);
                    }
                });
                if toplevel.is_initial_configure_sent() {
                    toplevel.send_pending_configure();
                }
            }
        }
    }
}

impl SelectionHandler for UnitState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for UnitState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for UnitState {}
impl ServerDndGrabHandler for UnitState {}
impl OutputHandler for UnitState {}

impl XdgShellHandler for UnitState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Compact layout: the window fills the window area.
        let area = self.window_area();
        surface.with_pending_state(|state| {
            if let Some(area) = area {
                state.size = Some(area.size);
            }
            state.states.set(xdg_toplevel::State::Maximized);
        });
        let wl_surface = surface.wl_surface().clone();
        let window = Window::new_wayland_window(surface);
        let loc = area.map(|a| a.loc).unwrap_or_default();
        self.space.map_element(window.clone(), loc, true);
        let (title, app_id) = crate::toplevel::title_and_app_id(&window);
        let dh = self.display_handle.clone();
        self.toplevels.window_opened(
            &dh,
            window,
            crate::toplevel::ToplevelInfo { title, app_id, ..Default::default() },
        );
        self.set_keyboard_focus(Some(wl_surface));
        tracing::info!("window opened ({} open)", self.open_window_count());
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self.window_for_surface(surface.wl_surface()).or_else(|| {
            self.minimized
                .iter()
                .find(|w| w.toplevel().map(|t| t.wl_surface() == surface.wl_surface()).unwrap_or(false))
                .cloned()
        });
        if let Some(window) = window {
            self.space.unmap_elem(&window);
            self.minimized.retain(|w| w != &window);
            self.toplevels.window_closed(&window);
        }
        self.focus_top_window();
        tracing::info!("window closed ({} open)", self.open_window_count());
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        self.unconstrain_popup(&surface);
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    // In the compact layout windows are not moved or resized by the user.
    fn move_request(&mut self, _surface: ToplevelSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}

    fn resize_request(
        &mut self,
        _surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial,
        _edges: xdg_toplevel::ResizeEdge,
    ) {
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
}

impl UnitState {
    /// Windows open, including minimised ones.
    pub fn open_window_count(&self) -> usize {
        self.space.elements().count() + self.minimized.len()
    }

    /// Keyboard focus to the window now on top, or to nothing (the home screen shows).
    pub fn focus_top_window(&mut self) {
        let top = self.space.elements().last().and_then(|w| w.toplevel().map(|t| t.wl_surface().clone()));
        self.set_keyboard_focus(top);
    }

    /// Sizes every window, minimised ones included, to the window area and places it there.
    pub fn fit_windows(&mut self) {
        let Some(area) = self.window_area() else { return };
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        for window in windows.iter().chain(self.minimized.iter()) {
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| state.size = Some(area.size));
                if toplevel.is_initial_configure_sent() {
                    toplevel.send_pending_configure();
                }
            }
        }
        for window in windows {
            self.space.map_element(window, area.loc, false);
        }
    }

    /// Raises a window (bringing it back if minimised) and gives it keyboard focus.
    pub fn activate_window(&mut self, window: &Window) {
        let loc = self.window_area().map(|a| a.loc).unwrap_or_default();
        if let Some(pos) = self.minimized.iter().position(|w| w == window) {
            self.minimized.remove(pos);
            self.space.map_element(window.clone(), loc, true);
        } else {
            self.space.raise_element(window, true);
        }
        self.set_keyboard_focus(window.toplevel().map(|t| t.wl_surface().clone()));
    }

    /// Takes a window out of view (Home): it stays open and the switcher can bring it back.
    pub fn minimize_window(&mut self, window: &Window) {
        if self.space.elements().any(|w| w == window) {
            self.space.unmap_elem(window);
            self.minimized.push(window.clone());
            self.focus_top_window();
        }
    }

    /// What the window switcher is told about a window.
    pub fn toplevel_info(&self, window: &Window) -> crate::toplevel::ToplevelInfo {
        let (title, app_id) = crate::toplevel::title_and_app_id(window);
        let focus = self.seat.get_keyboard().and_then(|k| k.current_focus());
        crate::toplevel::ToplevelInfo {
            title,
            app_id,
            activated: focus.is_some() && window.toplevel().map(|t| t.wl_surface()) == focus.as_ref(),
            minimized: self.minimized.contains(window),
        }
    }

    /// Sends the window switcher what has changed. Called once per pass of the event loop.
    pub fn refresh_toplevels(&mut self) {
        let mut toplevels = std::mem::replace(&mut self.toplevels, crate::toplevel::ToplevelManagerState::empty());
        toplevels.refresh(|w| self.toplevel_info(w));
        self.toplevels = toplevels;
    }

    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|w| w.toplevel().map(|t| t.wl_surface() == surface).unwrap_or(false))
            .cloned()
    }

    fn unconstrain_popup(&self, popup: &PopupSurface) {
        let Ok(root) = find_popup_root_surface(&PopupKind::Xdg(popup.clone())) else {
            return;
        };
        let Some(window) = self.window_for_surface(&root) else {
            return;
        };
        let (Some(output_geo), Some(window_geo)) =
            (self.output_geometry(), self.space.element_geometry(&window))
        else {
            return;
        };
        let mut target = output_geo;
        target.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(popup.clone()));
        target.loc -= window_geo.loc;
        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}

delegate_compositor!(UnitState);
delegate_shm!(UnitState);
delegate_seat!(UnitState);
delegate_data_device!(UnitState);
delegate_output!(UnitState);
delegate_xdg_shell!(UnitState);
