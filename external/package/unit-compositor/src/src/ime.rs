//! Input methods (Software Architecture §4, input methods: Fcitx5 with the Wayland input-method
//! protocols). Applications ask for text input with text-input v3; the input method (Fcitx5) talks
//! to the compositor with input-method v2 and virtual-keyboard v1, and shows its candidate window
//! as an input-method popup beside the text cursor.
//!
//! Added for the Shell UI toolkit spike (UNIT-OS-WP-0010, issue #8, condition C2). It returns to
//! the compositor through a normal pull request only if the spike outcome keeps it.

use smithay::{
    delegate_input_method_manager, delegate_text_input_manager, delegate_virtual_keyboard_manager,
    desktop::{PopupKind, PopupManager},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, DisplayHandle},
    utils::{Logical, Rectangle},
    wayland::{
        input_method::{InputMethodHandler, InputMethodManagerState, PopupSurface},
        text_input::TextInputManagerState,
        virtual_keyboard::VirtualKeyboardManagerState,
    },
};

use crate::state::UnitState;

/// Creates the text-input, input-method and virtual-keyboard globals.
pub fn init(dh: &DisplayHandle) {
    TextInputManagerState::new::<UnitState>(dh);
    InputMethodManagerState::new::<UnitState, _>(dh, |_client| true);
    VirtualKeyboardManagerState::new::<UnitState, _>(dh, |_client| true);
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

    /// Geometry of the window whose text field has the input method, so the candidate window is
    /// placed beside the text cursor.
    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.space
            .elements()
            .find_map(|window| {
                (window.toplevel().map(|t| t.wl_surface() == parent).unwrap_or(false))
                    .then(|| window.geometry())
            })
            .unwrap_or_default()
    }
}

delegate_text_input_manager!(UnitState);
delegate_input_method_manager!(UnitState);
delegate_virtual_keyboard_manager!(UnitState);
