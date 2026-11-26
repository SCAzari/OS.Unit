//! Layer surfaces for the Shell UI (wlr-layer-shell; Software Architecture r1.16 §8: "Unit
//! protocols for the shell (layer surfaces ...)").
//!
//! The Shell UI draws its own surfaces on four layers around the application windows: the home
//! screen on the background layer, and the status bar, navigation bar, quick settings,
//! notification banners and the window switcher on the top and overlay layers. Only the Shell UI
//! may create them (clients.rs). A surface that keeps an exclusive zone (the status bar) shrinks
//! the area application windows fill; windows are resized when the zones change.

use smithay::{
    delegate_layer_shell,
    desktop::{layer_map_for_output, LayerSurface, PopupKind, WindowSurfaceType},
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    utils::SERIAL_COUNTER,
    wayland::{
        compositor::with_states,
        shell::{
            wlr_layer::{
                KeyboardInteractivity, Layer, LayerSurface as WlrLayerSurface, LayerSurfaceData,
                WlrLayerShellHandler, WlrLayerShellState,
            },
            xdg::PopupSurface,
        },
    },
};

use crate::state::UnitState;

impl WlrLayerShellHandler for UnitState {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        // One display in this version: a surface goes on the display it names, or the first.
        let output = output
            .as_ref()
            .and_then(smithay::output::Output::from_resource)
            .or_else(|| self.space.outputs().next().cloned());
        let Some(output) = output else {
            tracing::warn!("layer surface {} refused: no display", namespace);
            surface.send_close();
            return;
        };
        let mut map = layer_map_for_output(&output);
        if let Err(err) = map.map_layer(&LayerSurface::new(surface, namespace.clone())) {
            tracing::warn!("layer surface {}: {}", namespace, err);
        }
        tracing::info!("layer surface opened: {}", namespace);
    }

    fn new_popup(&mut self, _parent: WlrLayerSurface, popup: PopupSurface) {
        let _ = self.popups.track_popup(PopupKind::Xdg(popup));
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        let had_focus = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .map(|f| &f == surface.wl_surface())
            .unwrap_or(false);
        for output in self.space.outputs() {
            let mut map = layer_map_for_output(output);
            let found = map
                .layers()
                .find(|l| l.layer_surface() == &surface)
                .cloned();
            if let Some(layer) = found {
                tracing::info!("layer surface closed: {}", layer.namespace());
                map.unmap_layer(&layer);
            }
        }
        self.fit_windows();
        if had_focus {
            self.focus_top_window();
        }
    }
}

impl UnitState {
    /// After a layer surface commits: lays the layers out again, sends the first configure, and
    /// gives keyboard focus to a surface that asks for it exclusively (for example the launcher
    /// with its search field).
    pub fn layer_commit(&mut self, surface: &WlSurface) {
        let Some(output) = self.space.outputs().find(|o| {
            layer_map_for_output(o)
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .is_some()
        }).cloned() else {
            return;
        };
        let initial_configure_sent = with_states(surface, |states| {
            states
                .data_map
                .get::<LayerSurfaceData>()
                .map(|data| data.lock().unwrap().initial_configure_sent)
                .unwrap_or(true)
        });
        let (layer, zones_changed) = {
            let mut map = layer_map_for_output(&output);
            let before = map.non_exclusive_zone();
            map.arrange();
            let layer = map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .cloned();
            (layer, before != map.non_exclusive_zone())
        };
        let Some(layer) = layer else { return };
        if !initial_configure_sent {
            layer.layer_surface().send_configure();
        }
        if zones_changed {
            self.fit_windows();
        }
        let wants_focus = layer.cached_state().keyboard_interactivity == KeyboardInteractivity::Exclusive
            && matches!(layer.layer(), Layer::Top | Layer::Overlay);
        if wants_focus {
            let current = self.seat.get_keyboard().and_then(|k| k.current_focus());
            if current.as_ref() != Some(surface) {
                self.set_keyboard_focus(Some(surface.clone()));
            }
        }
    }

    /// Gives keyboard focus to the layer surface under a point if it takes keyboard input
    /// (pressing a text field in the Shell UI). Returns whether one did.
    pub fn focus_layer_at(&mut self, pos: smithay::utils::Point<f64, smithay::utils::Logical>) -> bool {
        let Some(output) = self.space.outputs().next().cloned() else { return false };
        let Some(output_loc) = self.space.output_geometry(&output).map(|g| g.loc) else {
            return false;
        };
        // Layers above the windows first; the layers below only where no window covers the point.
        let target = {
            let map = layer_map_for_output(&output);
            let local = pos - output_loc.to_f64();
            let above = [Layer::Overlay, Layer::Top]
                .iter()
                .find_map(|l| map.layer_under(*l, local).cloned());
            above.or_else(|| {
                if self.space.element_under(pos).is_some() {
                    None
                } else {
                    [Layer::Bottom, Layer::Background]
                        .iter()
                        .find_map(|l| map.layer_under(*l, local).cloned())
                }
            })
        };
        match target {
            Some(layer) => {
                if layer.can_receive_keyboard_focus() {
                    self.set_keyboard_focus(Some(layer.wl_surface().clone()));
                }
                true
            }
            None => false,
        }
    }

    pub fn set_keyboard_focus(&mut self, surface: Option<WlSurface>) {
        let serial = SERIAL_COUNTER.next_serial();
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, surface, serial);
        }
    }
}

delegate_layer_shell!(UnitState);
