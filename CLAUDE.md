# Working notes for Claude Code (OS.Unit)

This repository is public. Never add anything private or secret: no personal names, addresses,
contact details, customer research, credentials, tokens or keys. Refer to people by role.

## What this is

Unit OS, the operating system of Unit's modular computing and VR devices, built with Buildroot
(see README.md for the layout, components and make targets). The project is run under PRINCE2;
its records (plans, decisions, issues, trackers) live in a separate documentation repository and
are kept by the Project Manager, not in this repository. IDs take the form `UNIT-OS-<TYPE>-<NNNN>`
(for example UNIT-OS-WP-0009, UNIT-OS-ISS-0020). Put the relevant IDs in commit messages.

Roles you will be working for: the Executive, Senior User and Senior Supplier (one person holds
these). Approval of designs, merges and checkpoint evidence is theirs.

## Current work: UNIT-OS-WP-0009, Desktop Foundation - Increment 1

Goal: the Executive sees the Unit desktop working in emulation (the demonstration), before
hardware suppliers are approached. Testing on purchased hardware waits until then (UNIT-OS-DEC-0105
and DEC-0107); testing in emulation continues now.

Checkpoints: 4 Dec (systemd image, met), 18 Dec (compositor renders and takes touch and pointer
input), 15 Jan (integrated Shell UI: home screen, status bar, launcher), 29 Jan (demonstration).

Conditions set by the Executive:
1. Escalate before exceeding 168 hours of effort or the 12 Feb 2027 demonstration date.
2. Stay within the agreed functional scope. The design comes first: build only what the approved
   designs (Interface Design, Software Architecture) describe; raise a question if they do not cover it.
3. AI-generated code is reviewed for quality, security, maintainability and architectural compliance.
4. The Senior User formally accepts the demonstration before WP-0009 closes.

Open now:
- PR #5 (Unit compositor, first version) was merged into `main` on 24 Nov 2026 (`6b69f4f`), with
  review findings 1-6 fixed. Still open from that review:
  - Finding 6, two parts: the legal-info manifest row for unit-compositor does not name the crates
    archive (`unit-compositor-0.1.0-crates.tar.gz`), and a first build without network access fails
    because the crates are fetched at build time, not by `make source`.
  - Low findings 7-14 (PR #5 review comment), deferred. Finding 7 (the `unit.render_wait=0` switch
    in every image) must be done before any release image is built.
- UNIT-OS-ISS-0020: the emulated display showed partly drawn frames. Fix: wait for drawing when
  Smithay's `RenderFrameResult::needs_sync()` is true; no hardware planes on the emulated GPU so the
  screenshots show the whole picture. A/B evidence: `make display` runs one extra boot with
  `unit.render_wait=0`. Can close for emulation (24 Nov 2026): CI run 45297755440 on the merged
  code passed 3 cold boots under the strict picture check, and the A/B boot failed with a partly
  drawn start-up frame. Still to confirm on purchased hardware (DEC-0105, DEC-0107).
- UNIT-OS-ISS-0019: monitors reporting placeholder sizes (16 x 9 cm and similar) use the fallback
  density (Interface Design r1.20 §5.1). Both closure conditions met on 24 Nov 2026 (display check
  passed including the picture check; Senior User approved the screenshots): to be closed by the
  Project Manager.
- Known limitations of the first compositor: runs as root; one display only; compact layout only
  (the layout class still reflects the width after scaling).

## How to work here

- Build: `make` (first build takes hours; later builds are incremental). Emulation checks:
  `make smoke`, `make harness`, `make display`. This VM has no `/dev/kvm`, so QEMU runs in software:
  use `DISPLAY_BOOTS=1 DISPLAY_AB=0 make display` for quick checks.
- Always look at the screenshots in `results/display/` yourself before reporting that the display
  works. Log lines alone are not evidence that the right picture reached the screen.
- Compositor unit tests: `cd external/package/unit-compositor/src && cargo test --locked --release`.
- Dependencies are pinned (Cargo.lock, Buildroot versions). Rust is Buildroot's 1.88.
- Branch for each change and open a pull request; CI (`.github/workflows/build.yml`) must pass.
  Never push to `main` directly.
- This machine is also the CI build runner `unit-build-1`: do not touch the runner's `_work` folder,
  and avoid local builds while a CI build is running.
- Do not change anything outside this VM (Windows, VMware, other VMs) without asking.
- Do not name other companies' products in Unit design text (code, comments and dependency names are
  fine).
- Report decisions and findings that need recording to the Executive, so the Project Manager can
  log them; this repository does not hold the project logs.
