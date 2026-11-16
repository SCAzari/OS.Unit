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
# Image identity for the result records (PA06). The disk runs in snapshot mode, so the booted
# image is the hashed image; the hash is checked again after the run.
RH=$(sha256sum "$IMG/rootfs.ext4" | cut -d' ' -f1)
KH=$(sha256sum "$IMG/bzImage" | cut -d' ' -f1)
printf 'rootfs.ext4 %s\nbzImage %s\n' "$RH" "$KH" > "$RES/image-sha256.txt"
echo "UNIT-IMAGE: rootfs.ext4 sha256 $RH; bzImage sha256 $KH"
ACCEL="-accel tcg"
[ -w /dev/kvm ] && ACCEL="-accel kvm"
timeout ${HARNESS_TIMEOUT:-1200} qemu-system-x86_64 $ACCEL -cpu max -m 2048 -smp 2 -M pc -nographic -no-reboot \
	-kernel "$IMG/bzImage" \
	-drive file="$IMG/rootfs.ext4",if=virtio,format=raw,snapshot=on \
	-append "rootwait root=/dev/vda console=ttyS0 unit.harness=$SEL unit.rootfs_sha256=$RH unit.kernel_sha256=$KH" \
	-virtfs local,path="$RES",mount_tag=unitresults,security_model=none,id=results \
	-device virtio-gpu-pci -netdev user,id=n0 -device virtio-net-pci,netdev=n0 | tee "$LOG" || true
[ "$(sha256sum "$IMG/rootfs.ext4" | cut -d' ' -f1)" = "$RH" ] || { echo "Disk image changed during the run"; exit 1; }
# Upload guard (PA08): a trace file is kept only if its meta file records that it passed
# verification. Anything else named as a trace is removed before the results are uploaded.
removed=0
for t in $(find "$RES" -type f \( -name trace -o -name 'trace.*' -o -name rejected -o -name discard -o -name skipped \) ! -name '*.meta' ! -name '*.verify'); do
	if ! grep -q '"verification":"passed"' "$t.meta" 2>/dev/null; then rm -f "$t"; echo "Upload guard: removed unverified trace $t"; removed=$((removed + 1)); fi
done
echo "Upload guard: $removed unverified trace files removed"
n=0
for f in $(find "$RES" -name 'H*.json'); do python3 -m json.tool "$f" > /dev/null || { echo "Invalid result record: $f"; exit 1; }; n=$((n + 1)); done
echo "$n result records kept in $RES"
grep -q "UNIT-HARNESS: PASS" "$LOG" && { echo "Harness checks passed"; exit 0; }
echo "Harness checks failed"; exit 1
