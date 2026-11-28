#!/bin/sh
# Shell UI check (UNIT-OS-WP-0009): boot the emulation image with the Unit compositor and the
# Shell UI, wait for the guest check (unit-shell-check) to report the shell ready, take a
# screenshot of the home screen and check that its parts were drawn. Results go to results/shell/.
set -eu
IMG=${1:?usage: qemu-shell.sh <images dir>}
RES=${SHELL_RESULTS:-results/shell}
LOG=${SHELL_LOG:-shell.log}
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
dir="$RES/boot-1"
mkdir -p "$dir"; rm -f "$LOG" "$dir/qmp.sock" "$dir/serial.log" "$dir/check.txt" "$dir"/*.png "$dir"/*.ppm
timeout ${SHELL_TIMEOUT:-900} qemu-system-x86_64 $ACCEL -cpu max -m 3072 -smp 2 -M pc -no-reboot \
	-display none -vga none -serial file:"$dir/serial.log" -qmp unix:"$dir/qmp.sock",server=on,wait=off \
	-kernel "$IMG/bzImage" \
	-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
	-append "rootwait root=/dev/vda console=ttyS0 unit.shell_check=1" \
	-device virtio-gpu-pci,xres=720,yres=1560 \
	-device virtio-tablet-pci -device virtio-keyboard-pci -device virtio-multitouch-pci \
	-netdev user,id=n0 -device virtio-net-pci,netdev=n0 &
qpid=$!
python3 "$(dirname "$0")/qmp-shell.py" "$dir/qmp.sock" "$dir/serial.log" "$dir" || true
wait $qpid || true
cp "$dir/serial.log" "$LOG"
grep -a "UNIT-SHELL:" "$dir/serial.log" | sed 's/^\[ *[0-9.]*\] [^:]*: //' || true
cat "$dir/check.txt" 2>/dev/null || echo "CHECK: no result"
if grep -q "UNIT-SHELL: PASS" "$dir/serial.log" && grep -q "CHECK: PASS" "$dir/check.txt" 2>/dev/null; then
	echo "Shell UI check passed"; exit 0
fi
echo "Shell UI check failed"; exit 1
