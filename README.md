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
```

Output images are in `output/<config>/images/`. The first build compiles the toolchain-side tools, LLVM and Mesa and takes several hours; later builds use the download and compiler caches.

## Smoke check

Booting with `unit.smoke=1` on the kernel command line runs `/etc/init.d/S99unit-smoke`, which prints:
the Unit OS release and build, the kernel version, whether the Intel Xe3 firmware files are present, and the Vulkan device found, then `UNIT-SMOKE: PASS` or `FAIL`, and powers off.

## Licences

The image combines upstream open-source components under their own licences with Unit's own software.
`make legal-info` produces the licence manifest and the source of every component, which is kept with each build.
The Linux kernel is GPL-2.0: Unit publishes the kernel source it ships, with its changes, when it distributes an image.
Firmware binaries are redistributed unmodified under their linux-firmware licences.
The licence for Unit's own software is set under Design decision D-09.
