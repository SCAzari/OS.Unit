# trace-filter.awk: label, verify and bound a kernel trace (Performance procedure PA08).
# Used by unit-trace. POSIX awk (BusyBox awk).
#
# Variables (-v):
#   mode      "export": replace free-text names with fixed labels, verify, apply the budget and
#                       write the result to "out" only if there are no findings; the input is the
#                       live trace buffer, so nothing unverified is ever written to a file.
#             "check":  verify the input as it is (an exported file, or records on stdin).
#   enabled   events that were enabled ("group:event ...")
#   fields    permitted fields per event ("event=f1,f2 ...")
#   literals  permitted literal tokens per event ("event:token ...")
#   budget    byte budget of the exported file (export mode)
#   out       output file (export mode)
# Prints one JSON summary line; exits 1 on any finding.
#
# Labels: task names (record headers and the comm fields) and interrupt device names are free
# text, so they are replaced by labels from the fixed set below. Process numbers, interrupt
# numbers, CPU numbers, states and counters are kept.

function label(name) {
	if (name ~ /^(<idle>|swapper\/[0-9]+)$/) return "idle"
	if (name ~ /^kworker\//) return "kworker"
	if (name ~ /^ksoftirqd\/[0-9]+$/) return "ksoftirqd"
	if (name ~ /^migration\/[0-9]+$/) return "migration"
	if (name ~ /^cpuhp\/[0-9]+$/) return "cpuhp"
	if (name ~ /^rcu_/) return "rcu"
	if (name in harness_tool) return name
	return "task"
}
function is_label(v) { return (v in labels) }

BEGIN {
	n = split(enabled, e, " "); for (i = 1; i <= n; i++) { sub(/^[^:]*:/, "", e[i]); en[e[i]] = 1 }
	n = split(fields, fs, " ")
	for (i = 1; i <= n; i++) { split(fs[i], kv, "="); m = split(kv[2], fl, ","); for (j = 1; j <= m; j++) ok[kv[1] SUBSEP fl[j]] = 1 }
	n = split(literals, ls, " ")
	for (i = 1; i <= n; i++) { split(ls[i], kv, ":"); lit[kv[1] SUBSEP kv[2]] = 1 }
	n = split("unit-inject unit-evstamp unit-harness unit-trace", ht, " "); for (i = 1; i <= n; i++) harness_tool[ht[i]] = 1
	n = split("idle kworker ksoftirqd migration cpuhp rcu task unit-inject unit-evstamp unit-harness unit-trace", lb, " ")
	for (i = 1; i <= n; i++) labels[lb[i]] = 1
	labels["irq"] = 1
	# Free-text fields: the field, and the field that follows it in the trace text ("" = end of line).
	ft["sched_switch" SUBSEP "prev_comm"] = "prev_pid"; ft["sched_switch" SUBSEP "next_comm"] = "next_pid"
	ft["sched_wakeup" SUBSEP "comm"] = "pid"; ft["irq_handler_entry" SUBSEP "name"] = ""
	bytes = 0; kept = 0
}
/^#/ || /^[ \t]*$/ { next }
{
	lines++
	line = $0
	# Header: task-pid, optional tgid, [cpu], flags, timestamp, event name.
	if (!match(line, /^ *.+-[0-9]+ +(\([ 0-9-]+\) +)?\[[0-9]+\] +[^ ]+ +[0-9]+\.[0-9]+: [a-z_]+: /)) { bad_lines++; next }
	head = substr(line, 1, RLENGTH); rest = substr(line, RLENGTH + 1)
	match(head, / +(\([ 0-9-]+\) +)?\[[0-9]+\]/)
	taskpid = substr(head, 1, RSTART - 1); tail = substr(head, RSTART)
	sub(/^ +/, "", taskpid)
	task = taskpid; sub(/-[0-9]+$/, "", task); pid = substr(taskpid, length(task) + 2)
	ev = head; sub(/: $/, "", ev); sub(/.*: /, "", ev)
	if (mode == "export") {
		task = label(task)
		for (k in ft) {
			split(k, kk, SUBSEP); if (kk[1] != ev) continue
			f = kk[2]; nx = ft[k]
			if (nx == "") { if (match(rest, "(^| )" f "=.*$")) { p = (substr(rest, RSTART, 1) == " ") ? " " : ""; rest = substr(rest, 1, RSTART - 1) p f "=irq" } }
			else if (match(rest, "(^| )" f "=.* " nx "=")) {
				p = (substr(rest, RSTART, 1) == " ") ? " " : ""
				v = substr(rest, RSTART + length(p) + length(f) + 1, RLENGTH - length(p) - length(f) - 1 - length(nx) - 2)
				rest = substr(rest, 1, RSTART - 1) p f "=" label(v) " " nx "=" substr(rest, RSTART + RLENGTH)
			}
		}
		line = sprintf("%16s-%-7s%s%s", task, pid, tail, rest)
	}
	count[ev]++
	if (!(ev in en)) { bad_events++; badev[ev] = 1; next }
	if (!is_label(task)) { unlabelled++; badl[ev] = 1 }
	# Payload: every token must be a permitted field (each name once) or a permitted literal;
	# free-text fields must hold a label.
	r2 = rest; gsub(/, /, " ", r2)
	k = split(r2, tok, / /); delete seen
	for (j = 1; j <= k; j++) {
		t = tok[j]
		if (t == "") continue
		if ((ev SUBSEP t) in lit) continue
		if (t ~ /^[a-z_-]+=[^=]+$/ || t ~ /^[a-z_-]+=$/) {
			name = t; sub(/=.*/, "", name); val = t; sub(/^[^=]*=/, "", val)
			if (!((ev SUBSEP name) in ok)) { bad_fields++; badf[ev "." name] = 1; continue }
			if (name in seen) { bad_tokens++; badt[ev] = 1; continue }
			seen[name] = 1
			if ((ev SUBSEP name) in ft && !is_label(val)) { unlabelled++; badl[ev] = 1 }
			continue
		}
		bad_tokens++; badt[ev] = 1
	}
	if (mode == "export" && !truncated) {
		if (bytes + length(line) + 1 > budget) truncated = 1
		else { buf[++kept] = line; bytes += length(line) + 1 }
	}
}
END {
	findings = bad_events + bad_fields + bad_tokens + bad_lines + unlabelled
	written = 0
	if (mode == "export" && findings == 0 && kept > 0) {
		for (i = 1; i <= kept; i++) print buf[i] > out
		close(out); written = bytes
	}
	printf "{\"verification\":\"%s\",\"records\":%d,\"records_exported\":%d,\"written_bytes\":%d,\"truncated\":%s,\"events\":{", \
		findings ? "rejected" : "passed", lines, (mode == "export" && findings == 0) ? kept : 0, written, truncated ? "true" : "false"
	s = ""; for (x in count) { printf "%s\"%s\":%d", s, x, count[x]; s = "," }
	printf "},\"unexpected_events\":["; s = ""; for (x in badev) { printf "%s\"%s\"", s, x; s = "," }
	printf "],\"unexpected_fields\":["; s = ""; for (x in badf) { printf "%s\"%s\"", s, x; s = "," }
	printf "],\"unexplained_payload_tokens\":%d,\"events_with_unexplained_payload\":[", bad_tokens + 0; s = ""
	for (x in badt) { printf "%s\"%s\"", s, x; s = "," }
	printf "],\"unlabelled_names\":%d,\"events_with_unlabelled_names\":[", unlabelled + 0; s = ""
	for (x in badl) { printf "%s\"%s\"", s, x; s = "," }
	printf "],\"unparsed_records\":%d}\n", bad_lines + 0
	exit findings ? 1 : 0
}
