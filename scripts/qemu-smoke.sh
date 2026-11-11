#!/bin/sh
# Boot the Unit OS emulation image in QEMU and check the smoke results on the serial console.
set -eu
IMG=${1:?usage: qemu-smoke.sh <images dir>}
LOG=${SMOKE_LOG:-smoke.log}
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
timeout ${SMOKE_TIMEOUT:-600} qemu-system-x86_64 $ACCEL -cpu max -m 2048 -smp 2 -M pc -nographic -no-reboot \
	-kernel "$IMG/bzImage" \
	-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
	-append "rootwait root=/dev/vda console=ttyS0 unit.smoke=1" \
	-device virtio-gpu-pci -netdev user,id=n0 -device virtio-net-pci,netdev=n0 | tee "$LOG" || true
grep -q "UNIT-SMOKE: PASS" "$LOG" && { echo "Smoke check passed"; exit 0; }
echo "Smoke check failed"; exit 1
