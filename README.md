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
| `.github/workflows/build.yml` | CI: builds each configuration from a clean checkout on the self-hosted runner `unit-build-1` (label `unit-build`; a manual run can pick a GitHub-hosted machine instead), keeps the licence manifest, then runs the smoke check and test harness on a GitHub-hosted machine with KVM. |

## Upstream components in the current image

| Layer | Component | Version |
|---|---|---|
| Kernel | Linux (long-term 6.18 series) | 6.18.63 |
| Firmware | linux-firmware, Intel graphics (i915, xe) | Buildroot 2026.02.3 package |
| Graphics | Mesa: Intel Vulkan (ANV), Intel OpenGL (Iris), software Vulkan (lavapipe, for emulation) | 26.0.1 |
| Init and services | systemd, with dbus-broker as the message bus, systemd-logind and systemd-networkd (Software Architecture r1.16 §7) | 258.7 (Buildroot 2026.02.3 package) |
| User space tools | BusyBox | Buildroot 2026.02.3 package |
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

Booting with `unit.smoke=1` on the kernel command line starts `unit-smoke.service` (`/usr/libexec/unit/unit-smoke`), which prints:
the Unit OS release and build, the kernel version, that PID 1 is systemd and start-up finished with no failed unit, that the message bus is active,
whether the Intel Xe3 firmware files are present, and the Vulkan device found, then `UNIT-SMOKE: PASS` or `FAIL`, and powers off.
The test harness starts the same way from `unit-harness.service` with `unit.harness=<scenarios|all>`.

## Test harness and trace capture

`unit-harness` measures and records performance evidence under the os.unit Performance Acceptance Plan
(procedures PA06 result record, PA07 pass logic, PA08 efficient instrumentation) and the handoff events of the
os.unit D-06 Limits (E1: input reaches the Unit OS). It is for development and test images, not release images.

| Tool | Purpose |
|---|---|
| `unit-evstamp` | Records the kernel timestamp (E1) and read time of each key-down or touch contact. Never records key codes, values or coordinates, so PINs and typed text cannot appear in a trace. Bounded by event count and bytes. |
| `unit-inject` | Synthetic key presses through uinput, with the injection time of each, to check the timestamp chain. |
| `unit-trace` | Kernel event capture with a per-CPU ring-buffer size and a free-space check. Accepts only allow-listed scheduling, interrupt, display-timing and power events; disables every other event before starting and confirms that exactly the requested events are on. On stop, the live buffer is read once: task names and interrupt device names are replaced by fixed labels, every record is verified (enabled event, wholly permitted fields, labels only) and the result is held in memory up to the storage budget. The file is written only if verification passed; a rejected capture leaves only a meta file with counts and event and field names. Record headers are parsed field by field from the right (event, timestamp, flags, CPU, thread-group id, process number) within a bound set by the 15-character task-name limit, and the exported header is rebuilt from those fields only. `unit-trace verify` re-checks an exported file, or records on standard input; `unit-trace export - FILE` runs the export path on records from standard input for regression checks. |
| `unit-stats` | Count, median, p95 (nearest rank), maximum and mean. No samples gives `n: 0`, recorded as not assessed. |
| `unit-pair` | Pairs injected and recorded events: lost, reordered, and timestamps outside the injection-to-read window. |
| `unit-harness` | Runs the scenarios below and writes one JSON result record per scenario with the fields PA06 lists. |

Every tool writes only inside the run's workspace (`UNIT_WORKSPACE`): an output path that resolves outside it, or is a
link, is refused and nothing is written. Before upload, the host removes any trace file whose meta file does not record
that it passed verification. Each result record also holds the booted kernel and disk image hashes (computed
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
| H04 Bounded, isolated capture | Harness check | Events outside the allow list refused; an event left on beforehand is disabled; the exported trace holds only enabled events with wholly permitted payloads and labels in place of task and device names (the workload's own process name must not appear); the verifier rejects, each for its own reason and from standard input only, a disallowed event, unexplained payload text, a disallowed field, an unlabelled device name and an unlabelled task name; task names containing CPU-, flags-, timestamp- or thread-group-shaped text are exported with rebuilt, labelled headers and no fragment of the original name; a capture with an event switched on behind the tool is rejected and not written; trace stops at its storage budget; nothing written when free space is short; every tool refuses outputs in /root, above the workspace or through a link; no file outside the workspace created or changed |

Display-timing events (E2, vertical blank) are captured when a display stack presents frames; they are checked on the
development configuration, as emulation has no Unit shell yet.

## Licences

The image combines upstream open-source components under their own licences with Unit's own software.
`make legal-info` produces the licence manifest and the source of every component, which is kept with each build.
The Linux kernel is GPL-2.0: Unit publishes the kernel source it ships, with its changes, when it distributes an image.
Firmware binaries are redistributed unmodified under their linux-firmware licences.
The licence for Unit's own software is set under Design decision D-09.
