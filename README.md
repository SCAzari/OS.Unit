# Unit OS

Unit's own operating system for core.unit, built on the Linux kernel and selected upstream components
(os.unit Design decision D-03: Linux-based foundation, no existing distribution as a base).

This repository is the os.unit build system (work package UNIT-OS-WP-0002: build system, repository and CI).

## Layout

| Path | Contents |
|---|---|
| `buildroot/` | Buildroot 2026.02.3 (long-term support), pinned as a submodule. Builds the upstream layers from source. |
| `external/` | Unit's own layer (Buildroot external tree `UNIT`): configurations, board files and Unit packages. |
| `external/configs/` | Image configurations. `unit_qemu_x86_64` is the emulation image; the development configuration image (Intel Core Ultra, Panther Lake) follows when the hardware is bought. |
| `external/package/` | Unit packages. `unit-release` sets the Unit OS identity. Unit's system services, shell and application model are added here. |
| `scripts/qemu-smoke.sh` | Boots the emulation image in QEMU and checks the smoke results on the serial console. |
| `external/package/unit-harness/` | Test harness and trace capture (development and test images only). |
| `scripts/qemu-harness.sh` | Boots the emulation image with the harness and keeps its result records on the host. |
| `.github/workflows/build.yml` | CI: builds each configuration from a clean checkout, runs the smoke check and keeps the licence manifest. |

## Upstream components in the current image

| Layer | Component | Version |
|---|---|---|
| Kernel | Linux (long-term 6.18 series) | 6.18.63 |
| Firmware | linux-firmware, Intel graphics (i915, xe) | Buildroot 2026.02.3 package |
| Graphics | Mesa: Intel Vulkan (ANV), Intel OpenGL (Iris), software Vulkan (lavapipe, for emulation) | 26.0.1 |
| User space | BusyBox init and tools (placeholder until Unit's system services) | Buildroot 2026.02.3 package |
| Toolchain | Bootlin x86-64 glibc stable | Buildroot 2026.02.3 package |

Minimums for the Intel Arc B390 graphics: Linux 6.17 and Mesa 25.1.6 (core.unit D-02 Decision Record).

## Build

Host: a recent Linux machine with the Buildroot host packages (see the CI workflow), about 60 GB of disk.

```
git clone --recurse-submodules <this repository>
make                 # build the emulation image
make smoke           # boot it in QEMU and run the smoke check
make harness         # boot it in QEMU, run the test harness, keep results in results/harness/
```

Output images are in `output/<config>/images/`. The first build compiles the toolchain-side tools, LLVM and Mesa and takes several hours; later builds use the download and compiler caches.

## Smoke check

Booting with `unit.smoke=1` on the kernel command line runs `/etc/init.d/S99unit-smoke`, which prints:
the Unit OS release and build, the kernel version, whether the Intel Xe3 firmware files are present, and the Vulkan device found, then `UNIT-SMOKE: PASS` or `FAIL`, and powers off.

## Test harness and trace capture

`unit-harness` measures and records performance evidence under the os.unit Performance Acceptance Plan
(procedures PA06 result record, PA07 pass logic, PA08 efficient instrumentation) and the handoff events of the
os.unit D-06 Limits (E1: input reaches the Unit OS). It is for development and test images, not release images.

| Tool | Purpose |
|---|---|
| `unit-evstamp` | Records the kernel timestamp (E1) and read time of each key-down or touch contact. Never records key codes, values or coordinates, so PINs and typed text cannot appear in a trace. Bounded by event count and bytes. |
| `unit-inject` | Synthetic key presses through uinput, with the injection time of each, to check the timestamp chain. |
| `unit-trace` | Kernel event capture with a per-CPU ring-buffer size, a storage budget for the file written (cut at a record boundary), and a free-space check before writing. Accepts only allow-listed scheduling, interrupt, display-timing and power events; disables every other event before starting and confirms that exactly the requested events are on. `unit-trace verify` checks that an exported trace holds only enabled, allow-listed events whose payload consists wholly of that event's permitted fields (each once) and permitted literals; any other payload text is rejected. |
| `unit-stats` | Count, median, p95 (nearest rank), maximum and mean. No samples gives `n: 0`, recorded as not assessed. |
| `unit-pair` | Pairs injected and recorded events: lost, reordered, and timestamps outside the injection-to-read window. |
| `unit-harness` | Runs the scenarios below and writes one JSON result record per scenario with the fields PA06 lists. |

Every tool writes only inside the run's workspace (`UNIT_WORKSPACE`): an output path that resolves outside it, or is a
link, is refused and nothing is written. Each result record also holds the booted kernel and disk image hashes (computed
by the host before boot; the disk runs in snapshot mode), a pinned workload identifier and revision, and an uncertainty
statement.

Each metric carries its target, provenance and approval reference. Following PA07, a functional failure is reported
first and fails the scenario and the run, whatever the timing. A valid measurement is *Not assessed* when the target or
its approval is missing. Until the D-06 limits are approved, every valid measurement is recorded as not assessed.

| Scenario | Kind | What it shows |
|---|---|---|
| H01 Input timestamp chain | Harness check | 200 synthetic presses: none lost or reordered; every E1 timestamp between injection and read; no key codes recorded |
| H02 Start-up stages | Measurement | Kernel start to init and to harness ready (emulation boots the kernel directly; D06-20 starts at the boot loader) |
| H03 Instrumentation overhead | Measurement | Change in median input read latency with tracing on, three alternating pairs, with the spread (method of D06-29) |
| H04 Bounded, isolated capture | Harness check | Events outside the allow list refused; an event left on beforehand is disabled; the exported trace holds only enabled events with wholly permitted payloads; the verifier rejects, each for its own reason, a disallowed event, unexplained payload text in an allowed event, and a disallowed field; trace stops at its storage budget; nothing written when free space is short; every tool refuses outputs in /root, above the workspace or through a link; no file outside the workspace created or changed |

Display-timing events (E2, vertical blank) are captured when a display stack presents frames; they are checked on the
development configuration, as emulation has no Unit shell yet.

## Licences

The image combines upstream open-source components under their own licences with Unit's own software.
`make legal-info` produces the licence manifest and the source of every component, which is kept with each build.
The Linux kernel is GPL-2.0: Unit publishes the kernel source it ships, with its changes, when it distributes an image.
Firmware binaries are redistributed unmodified under their linux-firmware licences.
The licence for Unit's own software is set under Design decision D-09.
