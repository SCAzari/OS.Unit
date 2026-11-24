#!/usr/bin/env python3
"""Host side of the toolkit spike tests (UNIT-OS-WP-0010, issue #8). Follows the guest's
UNIT-SPIKE lines on the serial log, sends switch keys, typing, taps and drags through QMP, takes
screenshots, and writes the boot's results (results.json) for the results record.

  qmp-spike.py <qmp socket> <serial log> <results dir> <toolkit> <screen> <test>
"""
import json, os, re, socket, statistics, struct, sys, time, zlib

qmp_path, log_path, res, toolkit, screen, test = sys.argv[1:7]
W, H = 720, 1560
FACTOR = 720 / 412            # emulated display: 412 points wide (display.conf)
SPEC = json.load(open(os.path.join(os.path.dirname(__file__), "..", "spike", "screens.json")))


def serial():
    try:
        with open(log_path, errors="replace") as f:
            return f.read()
    except FileNotFoundError:
        return ""


def wait_for(pattern, timeout):
    """First match of a regex on the serial log, or None after timeout (or when the guest is done)."""
    end = time.time() + timeout
    while time.time() < end:
        data = serial()
        m = re.search(pattern, data)
        if m:
            return m
        if "UNIT-SPIKE: DONE" in data:
            return None
        time.sleep(0.5)
    return None


class Qmp:
    def __init__(self, path):
        for _ in range(120):
            try:
                self.s = socket.socket(socket.AF_UNIX)
                self.s.connect(path)
                break
            except OSError:
                time.sleep(1)
        else:
            raise SystemExit("QMP socket not available")
        self.f = self.s.makefile("rw")
        self.f.readline()
        self.cmd("qmp_capabilities")

    def cmd(self, name, **args):
        self.f.write(json.dumps({"execute": name, "arguments": args}) + "\n")
        self.f.flush()
        while True:
            reply = json.loads(self.f.readline())
            if "return" in reply or "error" in reply:
                if "error" in reply:
                    print("QMP", name, "error:", reply["error"].get("desc"))
                return reply


def ppm_to_png(ppm, png):
    with open(ppm, "rb") as f:
        data = f.read()
    parts, pos = [], 0
    while len(parts) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        start = pos
        while not data[pos:pos + 1].isspace():
            pos += 1
        parts.append(data[start:pos])
    pos += 1
    w, h = int(parts[1]), int(parts[2])
    pixels = data[pos:pos + w * h * 3]
    raw = b"".join(b"\x00" + pixels[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    with open(png, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))
    os.remove(ppm)


def shot(q, name):
    ppm = os.path.abspath(os.path.join(res, name + ".ppm"))
    q.cmd("screendump", filename=ppm)
    time.sleep(1)
    if os.path.exists(ppm):
        ppm_to_png(ppm, ppm[:-4] + ".png")
        print("screenshot", name + ".png")


def key(q, qcode, shift=False):
    k = lambda down, c: {"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": c}}}
    ev = ([k(True, "shift")] if shift else []) + [k(True, qcode)]
    q.cmd("input-send-event", events=ev)
    time.sleep(0.05)
    ev = [k(False, qcode)] + ([k(False, "shift")] if shift else [])
    q.cmd("input-send-event", events=ev)


def to_abs(x_pts, y_pts):
    return int(x_pts * FACTOR / W * 32767), int(y_pts * FACTOR / H * 32767)


mtt = lambda kind, axis, value, tid=1: {"type": "mtt", "data": {"type": kind, "slot": 0,
                                         "tracking-id": tid, "axis": axis, "value": value}}


def tap(q, x_pts, y_pts, hold=0.08):
    x, y = to_abs(x_pts, y_pts)
    q.cmd("input-send-event", events=[mtt("begin", "x", x), mtt("data", "x", x), mtt("data", "y", y)])
    time.sleep(hold)
    q.cmd("input-send-event", events=[mtt("end", "x", x, -1)])


def extents(name):
    m = wait_for(r'PROBE EXTENTS "%s" (-?\d+) (-?\d+) (\d+) (\d+)' % re.escape(name), 240)
    if not m:
        return None
    x, y, w, h = map(int, m.groups())
    return x + w / 2, y + h / 2, w, h


result = {"toolkit": toolkit, "screen": screen, "test": test}


def finish():
    wait_for(r"UNIT-SPIKE: DONE", 900)
    data = serial()
    result["probe"] = [l.strip() for l in data.splitlines() if "PROBE C" in l or "C4A-SELECT" in l
                       or "PROBE SCAN" in l or "PROBE TEXT" in l or "UNIT-SPIKE: FAIL" in l]
    focus = [l.split("PROBE FOCUS ", 1)[1].strip() for l in data.splitlines()
             if "PROBE FOCUS " in l and "listening" not in l]
    if focus:
        result["focus"] = focus
    timing = [dict(re.findall(r"(\w+)=(\d+)", l)) for l in data.splitlines()
              if "UNIT-SPIKE-T: touch-timing: id=" in l and "input_us" in l]
    if timing:
        lat = sorted(int(t["to_present_us"]) / 1000 for t in timing)
        com = sorted(int(t["to_commit_us"]) / 1000 for t in timing)
        p95 = lambda v: v[min(len(v) - 1, int(round(0.95 * (len(v) - 1))))]
        result["touch_to_frame_ms"] = {"n": len(lat), "median": round(statistics.median(lat), 1),
                                       "p95": round(p95(lat), 1), "max": round(max(lat), 1)}
        result["touch_to_commit_ms"] = {"n": len(com), "median": round(statistics.median(com), 1),
                                        "p95": round(p95(com), 1), "max": round(max(com), 1)}
    commits = [int(m) for m in re.findall(r"UNIT-SPIKE-T: touch-timing: commit id=\d+ t_us=(\d+)", data)]
    if len(commits) > 2:
        gaps = sorted((b - a) / 1000 for a, b in zip(commits, commits[1:]) if 0 < b - a < 500_000)
        if gaps:
            p95 = gaps[min(len(gaps) - 1, int(round(0.95 * (len(gaps) - 1))))]
            result["drag_frame_interval_ms"] = {"n": len(gaps), "median": round(statistics.median(gaps), 1),
                                                "p95": round(p95, 1), "max": round(max(gaps), 1)}
    with open(os.path.join(res, "results.json"), "w") as f:
        json.dump(result, f, indent=1, ensure_ascii=False)
    print(json.dumps(result, ensure_ascii=False))


if not wait_for(r"UNIT-SPIKE: window", 900):
    print("spike window did not appear")
    result["error"] = "no window"
    finish()
    sys.exit(1)
q = Qmp(qmp_path)
time.sleep(4)
shot(q, f"1-{screen}")

if test == "a11y":
    scan = SPEC["screens"][screen]["scan"]
    press = SPEC["screens"][screen]["select"]["press"]
    if wait_for(r"UNIT-SPIKE: focus-ready", 600):
        # Switch "Next" (Tab) once round the declared order and back to its first element...
        for i in range(len(scan) + 1):
            key(q, "tab")
            time.sleep(0.8)
            if i in (0, 1, len(scan) - 1):
                shot(q, f"2-scan-{i + 1:02d}")
        # ...then on to the element to select, and "Select" (Space).
        for i in range(scan.index(press)):
            key(q, "tab")
            time.sleep(0.6)
        shot(q, "3-before-select")
        key(q, "spc")
        time.sleep(2)
        shot(q, "4-after-select")
    if screen == "settings" and wait_for(r"PROBE C1 display", 600):
        shot(q, "5-display")

elif test == "ime":
    field = extents("Search settings (test field)")
    if field and wait_for(r"UNIT-SPIKE: ime-ready", 60):
        tap(q, field[0], field[1])
        time.sleep(1.5)
        # Candidate window: "kanji", convert, open the candidate list, then cancel.
        for c in "kanji":
            key(q, c); time.sleep(0.15)
        time.sleep(1); shot(q, "2-preedit-kanji")
        key(q, "spc"); time.sleep(1.5)
        key(q, "spc"); time.sleep(1.5); shot(q, "3-candidates")
        key(q, "esc"); time.sleep(0.5); key(q, "esc"); time.sleep(0.5)
        for _ in range(6):
            key(q, "backspace"); time.sleep(0.1)
        # The test word: "nihongo", convert, commit.
        for c in "nihongo":
            key(q, c); time.sleep(0.15)
        time.sleep(1); shot(q, "4-preedit")
        key(q, "spc"); time.sleep(1.5); shot(q, "5-converted")
        key(q, "ret"); time.sleep(1.5); shot(q, "6-committed")

elif test == "touch-tap":
    k = extents("5")
    if k and wait_for(r"UNIT-SPIKE: touch-ready", 60):
        for i in range(200):
            tap(q, k[0], k[1])
            time.sleep(0.3)     # each tap draws the key's pressed state, so every tap changes the frame
        shot(q, "2-after-taps")

elif test == "touch-drag":
    row = extents("Display and appearance")
    if row and wait_for(r"UNIT-SPIKE: drag-open", 60):
        tap(q, row[0], row[1])
        s = extents("Corner rounding")
        if s and wait_for(r"UNIT-SPIKE: drag-ready", 120):
            cx, cy, w, _ = s
            x0, x1 = cx - w / 2 + 16, cx + w / 2 - 16
            ax, ay = to_abs(x0, cy)
            q.cmd("input-send-event", events=[mtt("begin", "x", ax), mtt("data", "x", ax), mtt("data", "y", ay)])
            for cycle in range(10):
                for step in range(40):
                    f = step / 39 if cycle % 2 == 0 else 1 - step / 39
                    ax, _ = to_abs(x0 + (x1 - x0) * f, cy)
                    q.cmd("input-send-event", events=[mtt("data", "x", ax)])
                    time.sleep(0.016)
            q.cmd("input-send-event", events=[mtt("end", "x", ax, -1)])
            time.sleep(1)
            shot(q, "2-after-drag")

finish()
