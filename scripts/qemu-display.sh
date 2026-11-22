#!/bin/sh
# Boot the Unit OS emulation image with a virtual GPU, tablet, keyboard and touchscreen, wait for the
# Unit compositor, take screenshots and send pointer and touch input through QMP, and check that the
# compositor received the input (UNIT-OS-WP-0009). Screenshots go to results/display/.
set -eu
IMG=${1:?usage: qemu-display.sh <images dir>}
RES=${DISPLAY_RESULTS:-results/display}
LOG=${DISPLAY_LOG:-display.log}
mkdir -p "$RES"
QMP="$RES/qmp.sock"
rm -f "$QMP" "$LOG"
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
timeout ${DISPLAY_TIMEOUT:-900} qemu-system-x86_64 $ACCEL -cpu max -m 3072 -smp 2 -M pc -no-reboot \
	-display none -vga none -serial file:"$LOG" -qmp unix:"$QMP",server=on,wait=off \
	-kernel "$IMG/bzImage" \
	-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
	-append "rootwait root=/dev/vda console=ttyS0 unit.display_check=1" \
	-device virtio-gpu-pci,xres=720,yres=1560 \
	-device virtio-tablet-pci -device virtio-keyboard-pci -device virtio-multitouch-pci \
	-netdev user,id=n0 -device virtio-net-pci,netdev=n0 &
QPID=$!
python3 "$(dirname "$0")/qmp-display.py" "$QMP" "$LOG" "$RES" || true
wait $QPID || true
cat "$LOG"
grep -q "UNIT-DISPLAY: PASS" "$LOG" && { echo "Display check passed"; exit 0; }
echo "Display check failed"; exit 1
