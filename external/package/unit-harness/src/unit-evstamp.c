/*
 * unit-evstamp: record input event timestamps (handoff event E1 in os.unit D-06 Limits).
 *
 * Reads an evdev device with CLOCK_MONOTONIC timestamps and writes one line per
 * key-down or touch-contact event:
 *     <seq> <kernel timestamp ns> <read timestamp ns>
 * The kernel timestamp is the time the input core received the report (E1).
 * The read timestamp is when this process received it.
 *
 * Privacy (Performance procedure PA08): key codes, values and coordinates are
 * never written, so typed PINs, passwords and dictation cannot appear in a trace.
 *
 * Bounds (PA08): stops after --max events or when the output reaches --budget
 * bytes, and reports which limit stopped it. Writes only its output file, which must lie
 * inside UNIT_WORKSPACE and is not opened through a link.
 *
 * Usage: unit-evstamp --dev /dev/input/eventN --out FILE [--max N] [--budget BYTES]
 *        [--timeout-ms MS]
 * Prints "READY" on stdout once the device is open.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <fcntl.h>
#include <linux/input.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t stop;
static void on_signal(int s) { (void)s; stop = 1; }

/* Workspace boundary (PA08): the output file's directory must resolve inside UNIT_WORKSPACE
 * (default /var/lib/unit-test/results) and the file is opened without following links. */
static FILE *open_in_workspace(const char *out)
{
	const char *ws = getenv("UNIT_WORKSPACE");
	if (!ws || !*ws) ws = "/var/lib/unit-test/results";
	char wr[PATH_MAX], dr[PATH_MAX], dir[PATH_MAX];
	snprintf(dir, sizeof dir, "%s", out);
	char *slash = strrchr(dir, '/');
	if (slash == dir) dir[1] = 0; else if (slash) *slash = 0; else snprintf(dir, sizeof dir, ".");
	if (!realpath(ws, wr) || !realpath(dir, dr)) { errno = ENOENT; return NULL; }
	size_t n = strlen(wr);
	if (strncmp(dr, wr, n) != 0 || (dr[n] != '/' && dr[n] != 0)) { errno = EACCES; return NULL; }
	int fd = open(out, O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW | O_CLOEXEC, 0644);
	if (fd < 0) return NULL;
	return fdopen(fd, "w");
}

static long long now_ns(void)
{
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return (long long)ts.tv_sec * 1000000000LL + ts.tv_nsec;
}

int main(int argc, char **argv)
{
	const char *dev = NULL, *out = NULL;
	long max = 1000, budget = 1 << 20, timeout_ms = 60000;
	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--dev") && i + 1 < argc) dev = argv[++i];
		else if (!strcmp(argv[i], "--out") && i + 1 < argc) out = argv[++i];
		else if (!strcmp(argv[i], "--max") && i + 1 < argc) max = atol(argv[++i]);
		else if (!strcmp(argv[i], "--budget") && i + 1 < argc) budget = atol(argv[++i]);
		else if (!strcmp(argv[i], "--timeout-ms") && i + 1 < argc) timeout_ms = atol(argv[++i]);
		else { fprintf(stderr, "unit-evstamp: unknown argument %s\n", argv[i]); return 2; }
	}
	if (!dev || !out || max <= 0 || budget <= 0) {
		fprintf(stderr, "usage: unit-evstamp --dev DEV --out FILE [--max N] [--budget BYTES] [--timeout-ms MS]\n");
		return 2;
	}
	FILE *f = open_in_workspace(out);
	if (!f) { perror("unit-evstamp: output outside the workspace or not writable"); return 1; }
	int fd = open(dev, O_RDONLY | O_CLOEXEC);
	if (fd < 0) { perror("unit-evstamp: open device"); return 1; }
	int clk = CLOCK_MONOTONIC;
	if (ioctl(fd, EVIOCSCLOCKID, &clk) < 0) { perror("unit-evstamp: EVIOCSCLOCKID"); return 1; }
	signal(SIGTERM, on_signal);
	signal(SIGINT, on_signal);
	printf("READY\n");
	fflush(stdout);

	long seq = 0, written = 0;
	const char *reason = "signal";
	long long deadline = now_ns() + timeout_ms * 1000000LL;
	struct pollfd p = { .fd = fd, .events = POLLIN };
	while (!stop) {
		long long left = (deadline - now_ns()) / 1000000LL;
		if (left <= 0) { reason = "timeout"; break; }
		int r = poll(&p, 1, (int)(left > 1000 ? 1000 : left));
		if (r < 0) { if (errno == EINTR) continue; perror("unit-evstamp: poll"); return 1; }
		if (r == 0) continue;
		struct input_event ev[64];
		ssize_t n = read(fd, ev, sizeof ev);
		long long rd = now_ns();
		if (n < 0) { if (errno == EINTR) continue; perror("unit-evstamp: read"); return 1; }
		for (size_t i = 0; i < (size_t)n / sizeof ev[0]; i++) {
			int contact = (ev[i].type == EV_KEY && ev[i].value == 1);
			if (!contact) continue;
			long long kt = (long long)ev[i].input_event_sec * 1000000000LL + (long long)ev[i].input_event_usec * 1000LL;
			char line[96];
			int len = snprintf(line, sizeof line, "%ld %lld %lld\n", seq, kt, rd);
			if (written + len > budget) { reason = "budget"; stop = 1; break; }
			fputs(line, f);
			written += len;
			if (++seq >= max) { reason = "max"; stop = 1; break; }
		}
	}
	fclose(f);
	close(fd);
	fprintf(stderr, "unit-evstamp: %ld events, %ld bytes, stopped by %s\n", seq, written, reason);
	printf("STOPPED %s %ld\n", reason, seq);
	return 0;
}
