#!/bin/sh
# Toolkit spike tests (UNIT-OS-WP-0010, issue #8): boot the spike image once per toolkit, screen and
# test, drive it through QMP (scripts/qmp-spike.py) and collect results in results/spike/.
#
#   qemu-spike.sh <images dir>                 all boots
#   SPIKE_ONLY="qt-a11y-sign-in" qemu-spike.sh ...   chosen boots (space-separated names)
set -eu
IMG=${1:?usage: qemu-spike.sh <images dir>}
RES=${SPIKE_RESULTS:-results/spike}
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
mkdir -p "$RES"

boot() { # toolkit, screen, test, [extra kernel options]
	name="$1-$3-$2${4:+-im}"
	case " ${SPIKE_ONLY:-$name} " in *" $name "*) ;; *) return 0 ;; esac
	dir="$RES/$name"; log="$dir/serial.log"
	rm -rf "$dir"; mkdir -p "$dir"
	# No key repeat: slow emulation can deliver a key release late (see unit-compositor state.rs).
	opts="unit.spike=$1 unit.spike_screen=$2 unit.spike_test=$3 unit.key_repeat=0 ${4:-} ${SPIKE_EXTRA_OPTS:-}"
	case "$3" in touch-*) opts="$opts unit.touch_timing=1" ;; esac
	echo "===== $name ($opts)"
	timeout ${SPIKE_TIMEOUT:-1800} qemu-system-x86_64 $ACCEL -cpu max -m 3072 -smp 2 -M pc -no-reboot \
		-display none -vga none -serial file:"$log" -qmp unix:"$dir/qmp.sock",server=on,wait=off \
		-kernel "$IMG/bzImage" \
		-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
		-append "rootwait root=/dev/vda console=ttyS0 $opts" \
		-device virtio-gpu-pci,xres=720,yres=1560 \
		-device virtio-tablet-pci -device virtio-keyboard-pci -device virtio-multitouch-pci &
	qpid=$!
	python3 "$(dirname "$0")/qmp-spike.py" "$dir/qmp.sock" "$log" "$dir" "$1" "$2" "$3" || true
	wait $qpid || true
	grep -a -E "PROBE C|C4A-SELECT|PROBE SCAN .*order|PROBE TEXT|UNIT-SPIKE: (FAIL|toolkit)" "$log" || true
}

for tk in qt slint; do
	boot $tk sign-in a11y
	boot $tk settings a11y
	boot $tk display a11y
	boot $tk settings ime
	boot $tk sign-in touch-tap
	boot $tk settings touch-drag
done
boot qt settings ime unit.spike_im=fcitx
# Experiment: one AT-SPI text read of the empty field.
boot qt settings text-read
boot slint settings text-read
echo "Results in $RES"
