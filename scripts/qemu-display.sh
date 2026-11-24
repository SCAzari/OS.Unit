#!/bin/sh
# Boot the Unit OS emulation image with a virtual GPU, tablet, keyboard and touchscreen, wait for the
# Unit compositor, take screenshots and send pointer and touch input through QMP, and check the
# pictures and the input the compositor reported (UNIT-OS-WP-0009). Results go to results/display/.
#
# Cold boots: the check runs on DISPLAY_BOOTS separate boots (default 3); all must pass.
# A/B test (UNIT-OS-ISS-0020): one further boot with the compositor's render wait turned off
# (unit.render_wait=0), same build and plane settings. It is reported, not required to pass.
set -eu
IMG=${1:?usage: qemu-display.sh <images dir>}
RES=${DISPLAY_RESULTS:-results/display}
LOG=${DISPLAY_LOG:-display.log}
BOOTS=${DISPLAY_BOOTS:-3}
AB=${DISPLAY_AB:-1}
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
rm -f "$LOG"

boot() { # name, extra kernel options
	dir="$RES/$1"; log="$dir/serial.log"
	# Remove everything from an earlier run, screenshots included, so only this boot's pictures remain.
	mkdir -p "$dir"; rm -f "$dir/qmp.sock" "$log" "$dir/check.txt" "$dir"/*.png "$dir"/*.ppm
	timeout ${DISPLAY_TIMEOUT:-900} qemu-system-x86_64 $ACCEL -cpu max -m 3072 -smp 2 -M pc -no-reboot \
		-display none -vga none -serial file:"$log" -qmp unix:"$dir/qmp.sock",server=on,wait=off \
		-kernel "$IMG/bzImage" \
		-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
		-append "rootwait root=/dev/vda console=ttyS0 unit.display_check=1 $2" \
		-device virtio-gpu-pci,xres=720,yres=1560 \
		-device virtio-tablet-pci -device virtio-keyboard-pci -device virtio-multitouch-pci \
		-netdev user,id=n0 -device virtio-net-pci,netdev=n0 &
	qpid=$!
	python3 "$(dirname "$0")/qmp-display.py" "$dir/qmp.sock" "$log" "$dir" || true
	wait $qpid || true
	{ echo "===== $1 ($2) ====="; cat "$log"; } >> "$LOG"
	echo "===== $1 ${2:+($2)}"
	grep -a "UNIT-DISPLAY: \(layout\|.*layout:\|compositor\|PASS\|FAIL\)" "$log" | tail -n 4 || true
	cat "$dir/check.txt" 2>/dev/null || echo "CHECK: no result"
	grep -q "UNIT-DISPLAY: PASS" "$log" && grep -q "CHECK: PASS" "$dir/check.txt" 2>/dev/null
}

FAILED=0
i=1
while [ $i -le "$BOOTS" ]; do
	boot "boot-$i" "" || FAILED=$((FAILED + 1))
	i=$((i + 1))
done
if [ "$AB" = 1 ]; then
	if boot "ab-render-wait-off" "unit.render_wait=0"; then
		echo "A/B: render wait off: pictures correct on this boot"
	else
		echo "A/B: render wait off: check failed (expected if the render wait is the fix)"
	fi
fi
[ $FAILED -eq 0 ] && { echo "Display check passed on $BOOTS cold boots"; exit 0; }
echo "Display check failed on $FAILED of $BOOTS cold boots"; exit 1
