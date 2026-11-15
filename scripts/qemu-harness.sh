#!/bin/sh
# Boot the Unit OS emulation image in QEMU, run the test harness (UNIT-OS-WP-0002) and keep
# its result records and raw data on the host through a 9p share.
# Usage: qemu-harness.sh <images dir> [scenarios, default all]
set -eu
IMG=${1:?usage: qemu-harness.sh <images dir> [scenarios]}
SEL=${2:-all}
RES=${HARNESS_RESULTS:-results/harness}
LOG=${HARNESS_LOG:-harness.log}
mkdir -p "$RES"
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
timeout ${HARNESS_TIMEOUT:-1200} qemu-system-x86_64 $ACCEL -cpu max -m 2048 -smp 2 -M pc -nographic -no-reboot \
	-kernel "$IMG/bzImage" \
	-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
	-append "rootwait root=/dev/vda console=ttyS0 unit.harness=$SEL" \
	-virtfs local,path="$RES",mount_tag=unitresults,security_model=none,id=results \
	-device virtio-gpu-pci -netdev user,id=n0 -device virtio-net-pci,netdev=n0 | tee "$LOG" || true
n=0
for f in $(find "$RES" -name 'H*.json'); do python3 -m json.tool "$f" > /dev/null || { echo "Invalid result record: $f"; exit 1; }; n=$((n + 1)); done
echo "$n result records kept in $RES"
grep -q "UNIT-HARNESS: PASS" "$LOG" && { echo "Harness checks passed"; exit 0; }
echo "Harness checks failed"; exit 1
