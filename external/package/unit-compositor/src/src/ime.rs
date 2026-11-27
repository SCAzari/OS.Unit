//! Input methods (Software Architecture §4, input methods: Fcitx5 with the Wayland input-method
//! protocols). Applications and the Shell UI ask for text input with text-input v3; the input
//! method talks to the compositor with input-method v2 and virtual-keyboard v1, and shows its
//! candidate window as an input-method popup just below the text cursor.
//!
//! Only the input method the compositor started may use input-method v2, which receives the text
//! typed into the focused field; virtual-keyboard v1 is limited to it and the Shell UI (the
//! on-screen keyboard). See clients.rs.

use smithay::{
    delegate_input_method_manager, delegate_text_input_manager, delegate_virtual_keyboard_manager,
    desktop::{layer_map_for_output, PopupKind, PopupManager, WindowSurfaceType},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, DisplayHandle},
    utils::{Logical, Rectangle},
    wayland::{
        input_method::{InputMethodHandler, InputMethodManagerState, PopupSurface},
        text_input::TextInputManagerState,
        virtual_keyboard::VirtualKeyboardManagerState,
    },
};

use crate::{
    clients::{allowed, Privilege},
    state::UnitState,
};

/// Creates the text-input, input-method and virtual-keyboard globals.
pub fn init(dh: &DisplayHandle) {
    TextInputManagerState::new::<UnitState>(dh);
    InputMethodManagerState::new::<UnitState, _>(dh, |client| allowed(client, Privilege::InputMethod));
    VirtualKeyboardManagerState::new::<UnitState, _>(dh, |client| {
        allowed(client, Privilege::VirtualKeyboard)
    });
}

impl InputMethodHandler for UnitState {
    fn new_popup(&mut self, surface: PopupSurface) {
        if let Err(err) = self.popups.track_popup(PopupKind::from(surface)) {
            tracing::warn!("input method popup: {}", err);
        }
    }

    fn popup_repositioned(&mut self, _surface: PopupSurface) {}

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        if let Some(parent) = surface.get_parent().map(|parent| parent.surface.clone()) {
            let _ = PopupManager::dismiss_popup(&parent, &PopupKind::from(surface));
        }
    }

    /// Geometry of the window or Shell UI layer surface whose text field has the input method, in
    /// its own coordinates (as Smithay's reference compositor gives it); the candidate window is
    /// placed from the text cursor rectangle, relative to that surface.
    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        if let Some(geo) = self.space.elements().find_map(|window| {
            (window.toplevel().map(|t| t.wl_surface() == parent).unwrap_or(false)).then(|| window.geometry())
        }) {
            return geo;
        }
        self.space
            .outputs()
            .next()
            .and_then(|output| {
                let map = layer_map_for_output(output);
                let layer = map.layer_for_surface(parent, WindowSurfaceType::TOPLEVEL)?;
                map.layer_geometry(layer).map(|geo| Rectangle::from_size(geo.size))
            })
            .unwrap_or_default()
    }
}

/// Places an input-method popup just below the text cursor rather than over it: Smithay puts it
/// at the corner of the cursor rectangle and leaves placement to the compositor. Called when the
/// popup's surface commits.
pub fn place_popup(popup: &PopupSurface) {
    let rect = popup.text_input_rectangle();
    popup.set_location((rect.loc.x, rect.loc.y + rect.size.h).into());
}

delegate_text_input_manager!(UnitState);
delegate_input_method_manager!(UnitState);
delegate_virtual_keyboard_manager!(UnitState);
