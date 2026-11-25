# Spike-only patched crates (UNIT-OS-WP-0010)

Toolkit spike branch only (`spike/toolkit`); never merged (Executive's conditions, 25 Nov 2026).

- `i-slint-backend-winit` 1.18.1 from crates.io, with two changes in `accesskit.rs`, marked
  "Unit OS toolkit spike (spike-only patch)":
  1. `Role::RadioButton` added to the roles that get AccessKit's Focus action, so radio buttons
     are focusable for assistive technology (AT-SPI Focusable state).
  2. A focus request from assistive technology uses `FocusReason::Programmatic` instead of
     `FocusReason::PointerClick`, so it is honoured by elements with `focus-on-click: false`.
  Licence: as the crate (see its Cargo.toml: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0
  OR LicenseRef-Slint-Software-3.0).
