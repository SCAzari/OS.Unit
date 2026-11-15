# unit-harness library: result records (Performance procedure PA06) and pass logic (PA07).
# Sourced by unit-harness. POSIX sh and awk only.

# JSON string, escaped.
js() { printf '"%s"' "$(printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr -d '\n\r\t')"; }
utc() { date -u +%Y-%m-%dT%H:%M:%SZ; }

# Value of a unit.* parameter on the kernel command line, or empty.
cmdarg() { tr ' ' '\n' < /proc/cmdline | awk -F= -v k="$1" '$1 == k { print substr($0, length(k) + 2); exit }'; }

# Harness source revision: the commit in the release string, e.g. "Unit OS 0.0.1 (1682e22)".
harness_rev() { sed -n 's/.*(\([0-9a-f]*\)).*/\1/p' /etc/unit-release 2>/dev/null | head -n 1; }

# Build and hardware manifest of the running system (PA06). The image hashes are computed by the
# host before boot (scripts/qemu-harness.sh) and passed on the kernel command line; QEMU runs the
# disk in snapshot mode, so the booted image is the hashed image.
manifest() {
	rel=$(cat /etc/unit-release 2>/dev/null || echo unknown)
	cpu=$(awk -F': ' '/^model name/ { print $2; exit }' /proc/cpuinfo 2>/dev/null)
	ncpu=$(grep -c '^processor' /proc/cpuinfo 2>/dev/null || echo 0)
	mem=$(awk '/^MemTotal/ { print $2 }' /proc/meminfo 2>/dev/null)
	prod=$(cat /sys/class/dmi/id/product_name 2>/dev/null || echo unknown)
	rh=$(cmdarg unit.rootfs_sha256); kh=$(cmdarg unit.kernel_sha256)
	printf '"build":{"release":%s,"kernel":%s,"cmdline":%s,"rootfs_sha256":%s,"kernel_image_sha256":%s,"harness_revision":%s},' \
		"$(js "$rel")" "$(js "$(uname -r)")" "$(js "$(cat /proc/cmdline)")" "$(js "${rh:-not provided}")" "$(js "${kh:-not provided}")" "$(js "$(harness_rev)")"
	printf '"hardware":{"configuration":%s,"machine":%s,"cpu":%s,"cpus":%s,"memory_kb":%s},' \
		"$(js "${UNIT_CONFIGURATION:-unknown}")" "$(js "$prod")" "$(js "$cpu")" "${ncpu:-0}" "${mem:-0}"
}

# Pinned workload description (PA06).
# workload ID REVISION PARAMETERS_JSON
workload() {
	printf '{"id":%s,"revision":%s,"harness_revision":%s,"parameters":%s}' "$(js "$1")" "$(js "$2")" "$(js "$(harness_rev)")" "$3"
}

# PA07 decision for one metric.
# decide VALUE TARGET APPROVAL FUNCTIONAL_OK [lower|upper]
# A failed functional check is reported first, whatever the measurement or target: the
# measurement cannot be trusted. Otherwise Not assessed when the value, the target or its
# approval reference is missing, and Pass or Fail against an approved target.
decide() {
	v=$1; t=$2; a=$3; ok=$4; dir=${5:-upper}
	if [ "$ok" != 1 ]; then echo "Fail: functional check"; return; fi
	if [ -z "$v" ] || [ "$v" = "null" ]; then echo "Not assessed: no measurement"; return; fi
	if [ -z "$t" ]; then echo "Not assessed: no target"; return; fi
	if [ -z "$a" ]; then echo "Not assessed: target not approved"; return; fi
	awk -v v="$v" -v t="$t" -v d="$dir" 'BEGIN { if ((d == "upper" && v <= t) || (d == "lower" && v >= t)) print "Pass"; else print "Fail" }'
}

# Scenario status for a measurement scenario (PA07): a functional failure fails the scenario;
# otherwise Not assessed until every limit it measures is approved.
# scenario_status FUNCTIONAL_OK
scenario_status() { if [ "$1" = 1 ]; then echo "Not assessed"; else echo "Fail: functional check"; fi; }

# metric NAME VALUE UNIT TARGET PROVENANCE APPROVAL FUNCTIONAL_OK [lower|upper]
# Prints one metric object with its PA07 decision.
metric() {
	st=$(decide "$2" "$4" "$6" "$7" "${8:-upper}")
	val=${2:-null}; tgt=${4:-null}
	printf '{"name":%s,"value":%s,"unit":%s,"target":%s,"provenance":%s,"approval":%s,"decision":%s}' \
		"$(js "$1")" "$val" "$(js "$3")" "$tgt" "$(js "$5")" "$(js "$6")" "$(js "$st")"
}

# record FILE TEST_ID TITLE KIND STATUS WORKLOAD_JSON STATS_JSON COUNTS_JSON METRICS_JSON FAULTS_JSON RAW_JSON INSTR_JSON UNCERTAINTY
# KIND is "harness check" (the harness verifying itself) or "measurement".
# Writes the PA06 record to FILE and prints it on one line with the UNIT-RESULT: prefix.
record() {
	f=$1
	{
		printf '{"test_id":%s,"title":%s,"kind":%s,"status":%s,' "$(js "$2")" "$(js "$3")" "$(js "$4")" "$(js "$5")"
		printf '"start":%s,"end":%s,' "$(js "${REC_START:-}")" "$(js "$(utc)")"
		manifest
		printf '"workload":%s,' "$6"
		printf '"settings":{"preset":%s},' "$(js "${UNIT_PRESET:-not set}")"
		printf '"battery_condition":%s,"ambient":%s,' "$(js "${UNIT_BATTERY:-not applicable (emulation)}")" "$(js "${UNIT_AMBIENT:-not recorded}")"
		printf '"instrumentation":%s,"uncertainty":%s,"raw_trace":%s,"counts":%s,"statistics":%s,"metrics":[%s],"faults":%s,' \
			"${12}" "$(js "${13}")" "${11}" "$8" "$7" "$9" "${10}"
		printf '"reviewer":%s}\n' "$(js "${UNIT_REVIEWER:-to be named}")"
	} > "$f"
	echo "UNIT-RESULT: $(cat "$f")"
}
