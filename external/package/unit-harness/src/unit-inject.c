/*
 * unit-inject: synthetic input for harness calibration (handoff event E0 stand-in).
 *
 * Creates a uinput key device, prints its event node, waits --start-ms, then sends
 * --count key presses --interval-ms apart. For each press it writes
 *     <seq> <injection timestamp ns>
 * taken from CLOCK_MONOTONIC immediately before the write. Pairing these with
 * unit-evstamp output checks the timestamp chain: every kernel timestamp must lie
 * between its injection time and its read time, with no events lost or reordered.
 *
 * The output file must lie inside UNIT_WORKSPACE and is not opened through a link.
 *
 * Usage: unit-inject --out FILE [--count N] [--interval-ms MS] [--start-ms MS]
 */
#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <dirent.h>
#include <fcntl.h>
#include <linux/uinput.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>

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

static void sleep_ms(long ms)
{
	struct timespec ts = { ms / 1000, (ms % 1000) * 1000000L };
	while (nanosleep(&ts, &ts) != 0) ;
}

static int emit(int fd, int type, int code, int value)
{
	struct input_event ev;
	memset(&ev, 0, sizeof ev);
	ev.type = type; ev.code = code; ev.value = value;
	return write(fd, &ev, sizeof ev) == (ssize_t)sizeof ev ? 0 : -1;
}

int main(int argc, char **argv)
{
	const char *out = NULL;
	long count = 100, interval = 20, start = 2000;
	for (int i = 1; i < argc; i++) {
		if (!strcmp(argv[i], "--out") && i + 1 < argc) out = argv[++i];
		else if (!strcmp(argv[i], "--count") && i + 1 < argc) count = atol(argv[++i]);
		else if (!strcmp(argv[i], "--interval-ms") && i + 1 < argc) interval = atol(argv[++i]);
		else if (!strcmp(argv[i], "--start-ms") && i + 1 < argc) start = atol(argv[++i]);
		else { fprintf(stderr, "unit-inject: unknown argument %s\n", argv[i]); return 2; }
	}
	if (!out || count <= 0 || interval < 0) {
		fprintf(stderr, "usage: unit-inject --out FILE [--count N] [--interval-ms MS] [--start-ms MS]\n");
		return 2;
	}
	FILE *f = open_in_workspace(out);
	if (!f) { perror("unit-inject: output outside the workspace or not writable"); return 1; }
	int fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK | O_CLOEXEC);
	if (fd < 0) { perror("unit-inject: open /dev/uinput"); return 1; }
	ioctl(fd, UI_SET_EVBIT, EV_KEY);
	ioctl(fd, UI_SET_KEYBIT, KEY_SPACE);
	struct uinput_setup us;
	memset(&us, 0, sizeof us);
	us.id.bustype = BUS_VIRTUAL;
	us.id.vendor = 0x554e; /* "UN" */
	us.id.product = 0x0001;
	strcpy(us.name, "unit-harness-inject");
	if (ioctl(fd, UI_DEV_SETUP, &us) < 0 || ioctl(fd, UI_DEV_CREATE) < 0) { perror("unit-inject: create device"); return 1; }

	char sysname[64] = "", path[160];
	if (ioctl(fd, UI_GET_SYSNAME(sizeof sysname), sysname) < 0) { perror("unit-inject: UI_GET_SYSNAME"); return 1; }
	snprintf(path, sizeof path, "/sys/devices/virtual/input/%s", sysname);
	char node[300] = "";
	for (int tries = 0; tries < 50 && !node[0]; tries++) {
		DIR *d = opendir(path);
		if (d) {
			struct dirent *e;
			while ((e = readdir(d)))
				if (!strncmp(e->d_name, "event", 5)) snprintf(node, sizeof node, "/dev/input/%s", e->d_name);
			closedir(d);
		}
		if (!node[0]) sleep_ms(20);
	}
	if (!node[0]) { fprintf(stderr, "unit-inject: event node not found\n"); return 1; }
	printf("NODE %s\n", node);
	fflush(stdout);

	sleep_ms(start);
	for (long i = 0; i < count; i++) {
		long long t = now_ns();
		if (emit(fd, EV_KEY, KEY_SPACE, 1) || emit(fd, EV_SYN, SYN_REPORT, 0)) { perror("unit-inject: write"); return 1; }
		fprintf(f, "%ld %lld\n", i, t);
		sleep_ms(interval / 2);
		emit(fd, EV_KEY, KEY_SPACE, 0);
		emit(fd, EV_SYN, SYN_REPORT, 0);
		sleep_ms(interval - interval / 2);
	}
	fclose(f);
	sleep_ms(200);
	ioctl(fd, UI_DEV_DESTROY);
	close(fd);
	printf("DONE %ld\n", count);
	return 0;
}
