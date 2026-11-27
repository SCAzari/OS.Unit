//! Input from libinput: keyboard, pointer (relative and absolute) and touch.
//!
//! Pressing or touching a window raises it and gives it keyboard focus. Only on a display-check
//! boot (unit.display_check=1) are presses, releases and touches logged with their positions, so
//! the emulation check can confirm that input reaches the compositor. Otherwise no input is
//! logged: the positions and timing of touches can reveal a PIN or typed text. Key codes are
//! never logged.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent, TouchEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
        touch::{DownEvent, MotionEvent as TouchMotionEvent, UpEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::state::UnitState;

impl UnitState {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                if let Some(keyboard) = self.seat.get_keyboard() {
                    keyboard.input::<(), _>(
                        self,
                        event.key_code(),
                        event.state(),
                        serial,
                        time,
                        |_, _, _| FilterResult::Forward,
                    );
                }
            }
            InputEvent::PointerMotion { event } => {
                let pos = self.pointer_location + event.delta();
                self.pointer_moved(pos, event.time_msec());
            }
            InputEvent::PointerMotionAbsolute { event } => {
                if let Some(geo) = self.output_geometry() {
                    let pos = event.position_transformed(geo.size) + geo.loc.to_f64();
                    self.pointer_moved(pos, event.time_msec());
                }
            }
            InputEvent::PointerButton { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let state = event.state();
                if state == ButtonState::Pressed {
                    if self.log_input {
                        tracing::info!(
                            "input: pointer press at ({:.0}, {:.0})",
                            self.pointer_location.x,
                            self.pointer_location.y
                        );
                    }
                    self.focus_window_at(self.pointer_location);
                } else if self.log_input {
                    tracing::info!("input: pointer release");
                }
                if let Some(pointer) = self.seat.get_pointer() {
                    pointer.button(
                        self,
                        &ButtonEvent {
                            button: event.button_code(),
                            state,
                            serial,
                            time: event.time_msec(),
                        },
                    );
                    pointer.frame(self);
                }
            }
            InputEvent::PointerAxis { event } => {
                let source = event.source();
                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    let amount = event
                        .amount(axis)
                        .unwrap_or_else(|| event.amount_v120(axis).unwrap_or(0.0) * 15.0 / 120.0);
                    if amount != 0.0 {
                        frame = frame.value(axis, amount);
                        if let Some(discrete) = event.amount_v120(axis) {
                            frame = frame.v120(axis, discrete as i32);
                        }
                    } else if source == AxisSource::Finger && event.amount(axis) == Some(0.0) {
                        frame = frame.stop(axis);
                    }
                }
                if let Some(pointer) = self.seat.get_pointer() {
                    pointer.axis(self, frame);
                    pointer.frame(self);
                }
            }
            InputEvent::TouchDown { event } => {
                let Some(geo) = self.output_geometry() else { return };
                let pos = event.position_transformed(geo.size) + geo.loc.to_f64();
                if self.log_input {
                    tracing::info!("input: touch down at ({:.0}, {:.0})", pos.x, pos.y);
                }
                self.focus_window_at(pos);
                let under = self.surface_under(pos);
                if let Some(touch) = self.seat.get_touch() {
                    touch.down(
                        self,
                        under,
                        &DownEvent {
                            slot: event.slot(),
                            location: pos,
                            serial: SERIAL_COUNTER.next_serial(),
                            time: event.time_msec(),
                        },
                    );
                }
            }
            InputEvent::TouchMotion { event } => {
                let Some(geo) = self.output_geometry() else { return };
                let pos = event.position_transformed(geo.size) + geo.loc.to_f64();
                let under = self.surface_under(pos);
                if let Some(touch) = self.seat.get_touch() {
                    touch.motion(
                        self,
                        under,
                        &TouchMotionEvent {
                            slot: event.slot(),
                            location: pos,
                            time: event.time_msec(),
                        },
                    );
                }
            }
            InputEvent::TouchUp { event } => {
                if self.log_input {
                    tracing::info!("input: touch up");
                }
                if let Some(touch) = self.seat.get_touch() {
                    touch.up(
                        self,
                        &UpEvent {
                            slot: event.slot(),
                            serial: SERIAL_COUNTER.next_serial(),
                            time: event.time_msec(),
                        },
                    );
                }
            }
            InputEvent::TouchFrame { .. } => {
                if let Some(touch) = self.seat.get_touch() {
                    touch.frame(self);
                }
            }
            InputEvent::TouchCancel { .. } => {
                if let Some(touch) = self.seat.get_touch() {
                    touch.cancel(self);
                }
            }
            _ => {}
        }
    }

    fn pointer_moved(&mut self, pos: Point<f64, Logical>, time: u32) {
        let pos = self.clamp_to_output(pos);
        self.pointer_location = pos;
        self.pointer_visible = true;
        let under = self.surface_under(pos);
        if let Some(pointer) = self.seat.get_pointer() {
            pointer.motion(
                self,
                under,
                &MotionEvent {
                    location: pos,
                    serial: SERIAL_COUNTER.next_serial(),
                    time,
                },
            );
            pointer.frame(self);
        }
    }

    fn clamp_to_output(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        match self.output_geometry() {
            Some(geo) => {
                // A display less than one point wide or high keeps the pointer at its origin
                // (f64::clamp panics if the maximum is below the minimum).
                let max_x = (geo.loc.x + (geo.size.w - 1).max(0)) as f64;
                let max_y = (geo.loc.y + (geo.size.h - 1).max(0)) as f64;
                (pos.x.clamp(geo.loc.x as f64, max_x), pos.y.clamp(geo.loc.y as f64, max_y)).into()
            }
            None => pos,
        }
    }

    /// Gives keyboard focus to what is under a point: a Shell UI layer surface above the windows
    /// that takes keyboard input, or else the window there, which is raised.
    fn focus_window_at(&mut self, pos: Point<f64, Logical>) {
        if self.focus_layer_at(pos) {
            return;
        }
        let serial = SERIAL_COUNTER.next_serial();
        let window = self.space.element_under(pos).map(|(w, _)| w.clone());
        if let Some(window) = window {
            self.space.raise_element(&window, true);
            let surface = window.toplevel().map(|t| t.wl_surface().clone());
            if let Some(keyboard) = self.seat.get_keyboard() {
                keyboard.set_focus(self, surface, serial);
            }
            self.space.elements().for_each(|w| {
                if let Some(t) = w.toplevel() {
                    t.send_pending_configure();
                }
            });
        }
    }
}
