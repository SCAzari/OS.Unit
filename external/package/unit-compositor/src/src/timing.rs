//! Touch response measurement for the Shell UI toolkit spike (UNIT-OS-WP-0010, issue #8,
//! condition C3). Active only with the kernel option unit.touch_timing=1; otherwise every call is
//! a no-op and nothing is logged.
//!
//! For each touch-down it logs, on the CLOCK_MONOTONIC base in microseconds:
//! - input: the kernel time of the touch-down (from libinput);
//! - commit: the first commit by the touched client after it (the client's response);
//! - present: the page flip of the first frame that contains that commit.
//!
//! While a finger is down it also logs each commit of the touched client, so the host can work
//! out the client's frame intervals while scrolling. Positions are never logged.

use smithay::{
    reexports::wayland_server::backend::ClientId,
    utils::{Clock, Monotonic},
};

pub struct TouchTiming {
    enabled: bool,
    clock: Clock<Monotonic>,
    next_id: u64,
    probe: Option<Probe>,
    /// The client under the finger while it is down, with its touch number.
    touched: Option<(u64, ClientId)>,
}

struct Probe {
    id: u64,
    input_us: u64,
    client: ClientId,
    commit_us: Option<u64>,
    frame: Option<u64>,
}

impl TouchTiming {
    pub fn new(enabled: bool) -> Self {
        if enabled {
            tracing::warn!("touch timing on (unit.touch_timing=1): test boot only");
        }
        Self { enabled, clock: Clock::new(), next_id: 0, probe: None, touched: None }
    }

    fn now_us(&self) -> u64 {
        self.clock.now().as_micros()
    }

    /// A finger went down on a surface of `client` at kernel time `input_us`.
    pub fn touch_down(&mut self, input_us: u64, client: Option<ClientId>) {
        if !self.enabled {
            return;
        }
        if let Some(old) = self.probe.take() {
            tracing::info!("touch-timing: id={} incomplete", old.id);
        }
        let Some(client) = client else {
            tracing::info!("touch-timing: touch-down on no client");
            return;
        };
        self.next_id += 1;
        self.touched = Some((self.next_id, client.clone()));
        self.probe = Some(Probe { id: self.next_id, input_us, client, commit_us: None, frame: None });
    }

    pub fn touch_up(&mut self) {
        self.touched = None;
    }

    /// A client committed a surface.
    pub fn commit(&mut self, client: &ClientId) {
        if !self.enabled {
            return;
        }
        let now = self.now_us();
        if let Some(probe) = self.probe.as_mut() {
            if &probe.client == client && probe.commit_us.is_none() {
                probe.commit_us = Some(now);
            }
        }
        if let Some((id, touched)) = &self.touched {
            if touched == client {
                tracing::info!("touch-timing: commit id={} t_us={}", id, now);
            }
        }
    }

    /// Frame number `frame` was queued for display.
    pub fn frame_queued(&mut self, frame: u64) {
        if let Some(probe) = self.probe.as_mut() {
            if probe.commit_us.is_some() && probe.frame.is_none() {
                probe.frame = Some(frame);
            }
        }
    }

    /// Frame number `frame` reached the display at `present_us` (monotonic), if the display
    /// reported a monotonic time; otherwise the time the flip was handled is used.
    pub fn presented(&mut self, frame: u64, present_us: Option<u64>) {
        if !self.enabled {
            return;
        }
        let present_us = present_us.unwrap_or_else(|| self.now_us());
        if self.probe.as_ref().and_then(|p| p.frame) == Some(frame) {
            let p = self.probe.take().expect("probe");
            let commit_us = p.commit_us.unwrap_or(present_us);
            tracing::info!(
                "touch-timing: id={} input_us={} commit_us={} present_us={} to_commit_us={} to_present_us={}",
                p.id,
                p.input_us,
                commit_us,
                present_us,
                commit_us.saturating_sub(p.input_us),
                present_us.saturating_sub(p.input_us)
            );
        }
    }
}
