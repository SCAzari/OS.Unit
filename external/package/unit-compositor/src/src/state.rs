//! Compositor state shared by the protocol handlers, the input handling and the display backend.

use std::{ffi::OsString, sync::Arc, time::Instant};

use smithay::{
    desktop::{layer_map_for_output, PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point, Rectangle},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::{
            wlr_layer::{Layer, WlrLayerShellState},
            xdg::XdgShellState,
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::udev::UdevData;

pub struct UnitState {
    pub start_time: Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,

    /// Application windows. In the compact layout every window fills the display and the most
    /// recently raised one is on top.
    pub space: Space<Window>,
    pub popups: PopupManager,
    /// Application windows the Shell UI has minimised (Home), most recently minimised last. They
    /// are out of the space, so not drawn and not under input, until activated again.
    pub minimized: Vec<Window>,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    /// Layer surfaces for the Shell UI (status bar, navigation bar, home, quick settings, banners).
    pub layer_shell_state: WlrLayerShellState,
    /// Window list and control for the Shell UI's window switcher.
    pub toplevels: crate::toplevel::ToplevelManagerState,
    pub shm_state: ShmState,
    #[allow(dead_code)] // keeps the xdg-output global alive
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<UnitState>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<UnitState>,

    /// Pointer position in the space, drawn as the cursor.
    pub pointer_location: Point<f64, Logical>,
    /// The pointer is drawn only after a pointer device has moved it; touch alone shows no cursor.
    pub pointer_visible: bool,

    /// Layout class of the display (Requirement OS-52). Increment 1 lays out every class as compact.
    pub layout_class: crate::layout::LayoutClass,

    /// Log pointer presses and touches with their positions. Only on a display-check boot
    /// (unit.display_check=1): positions and timing of touches can reveal a PIN or typed text,
    /// so they are never logged otherwise.
    pub log_input: bool,

    pub backend: Option<UdevData>,
}

/// Whether the kernel command line has this exact option (for example "unit.display_check=1").
pub fn kernel_option(option: &str) -> bool {
    std::fs::read_to_string("/proc/cmdline")
        .map(|c| c.split_whitespace().any(|o| o == option))
        .unwrap_or(false)
}

impl UnitState {
    pub fn new(event_loop: &mut EventLoop<UnitState>, display: Display<UnitState>) -> Self {
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new_with_filter::<Self, _>(&dh, |client| {
            crate::clients::allowed(client, crate::clients::Privilege::LayerShell)
        });
        let toplevels = crate::toplevel::ToplevelManagerState::new(&dh);
        crate::ime::init(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "seat0");
        // Keyboard layout from the system's layout data (xkeyboard-config). Without it the
        // compositor still runs with pointer and touch input, and says why there is no keyboard.
        if let Err(err) = seat.add_keyboard(Default::default(), 400, 30) {
            tracing::error!("no keyboard: the keyboard layout could not be compiled ({:?})", err);
        }
        seat.add_pointer();
        seat.add_touch();

        let listening_socket = ListeningSocketSource::new_auto().expect("Wayland socket");
        let socket_name = listening_socket.socket_name().to_os_string();
        let handle = event_loop.handle();
        handle
            .insert_source(listening_socket, |stream, _, state| {
                if let Err(err) = state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                {
                    tracing::warn!("client rejected: {}", err);
                }
            })
            .expect("Wayland socket source");
        handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: the display is never dropped while the event loop runs.
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .expect("Wayland display source");

        Self {
            start_time: Instant::now(),
            socket_name,
            display_handle: dh,
            space: Space::default(),
            popups: PopupManager::default(),
            minimized: Vec::new(),
            compositor_state,
            xdg_shell_state,
            layer_shell_state,
            toplevels,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
            pointer_location: (0.0, 0.0).into(),
            pointer_visible: false,
            layout_class: crate::layout::LayoutClass::Compact,
            log_input: kernel_option("unit.display_check=1"),
            backend: None,
        }
    }

    /// The surface under a point, and its location in the space: Shell UI layer surfaces above
    /// the windows (overlay, then top) first, then application windows, then the layers below
    /// them (bottom, then background, where the home screen is).
    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        if let Some(found) = self.layer_surface_under(pos, &[Layer::Overlay, Layer::Top]) {
            return Some(found);
        }
        if let Some(found) = self.space.element_under(pos).and_then(|(window, location)| {
            window
                .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(surface, p)| (surface, (p + location).to_f64()))
        }) {
            return Some(found);
        }
        self.layer_surface_under(pos, &[Layer::Bottom, Layer::Background])
    }

    /// The layer surface (with its popups) under a point on one of the given layers, in order.
    pub fn layer_surface_under(
        &self,
        pos: Point<f64, Logical>,
        layers: &[Layer],
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        let output = self.space.outputs().next()?;
        let output_loc = self.space.output_geometry(output)?.loc;
        let map = layer_map_for_output(output);
        layers.iter().find_map(|layer| {
            let surface = map.layer_under(*layer, pos - output_loc.to_f64())?;
            let geo = map.layer_geometry(surface)?;
            let loc = output_loc + geo.loc;
            surface
                .surface_under(pos - loc.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (s, (p + loc).to_f64()))
        })
    }

    /// Area of the display that application windows fill: the display less the zones that the
    /// Shell UI's layer surfaces keep for themselves (for example the status bar).
    pub fn window_area(&self) -> Option<Rectangle<i32, Logical>> {
        let output = self.space.outputs().next()?;
        let output_geo = self.space.output_geometry(output)?;
        let mut zone = layer_map_for_output(output).non_exclusive_zone();
        zone.loc += output_geo.loc;
        Some(if zone.size.w > 0 && zone.size.h > 0 { zone } else { output_geo })
    }

    /// Size of the display in the space (the first output), if one is connected.
    pub fn output_geometry(&self) -> Option<smithay::utils::Rectangle<i32, Logical>> {
        self.space
            .outputs()
            .next()
            .and_then(|output| self.space.output_geometry(output))
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
    /// Applications connect through the Wayland socket; the compositor starts the Shell UI and
    /// the input method with connections that carry their roles (see clients.rs).
    pub role: crate::clients::ClientRole,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
