#!/usr/bin/env python3
"""Host side of the display check: waits for 'UNIT-DISPLAY: ready' on the serial log, then takes
screenshots and sends pointer and touch input to the emulated machine through QMP, then checks the
pictures and the input positions the compositor reported. Writes PICTURE: and INPUT: lines and a
final CHECK: PASS or CHECK: FAIL to <results>/check.txt."""
import json, os, re, socket, struct, sys, time, zlib

qmp_path, log_path, res = sys.argv[1:4]

def wait_log(text, timeout):
    end = time.time() + timeout
    while time.time() < end:
        try:
            with open(log_path, errors="replace") as f:
                data = f.read()
            if text in data:
                return True
            if "UNIT-DISPLAY: FAIL" in data or "UNIT-DISPLAY: PASS" in data:
                return False
        except FileNotFoundError:
            pass
        time.sleep(1)
    return False

class Qmp:
    def __init__(self, path):
        for _ in range(60):
            try:
                self.s = socket.socket(socket.AF_UNIX)
                self.s.connect(path)
                break
            except OSError:
                time.sleep(1)
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
    """Minimal PPM (P6) to PNG conversion, so the screenshots open anywhere."""
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
                + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    os.remove(ppm)
    return w, h, pixels

# Emulated display (UNIT-OS-WP-0009): 720 x 1560 pixels at the density factor in the emulation
# image's /etc/unit/display.conf, so 412 x 892.7 points. Input positions use the absolute range
# 0..32767 across the display.
W, H, FACTOR = 720, 1560, 720 / 412
PTS_W, PTS_H = W / FACTOR, H / FACTOR
POINTER_PX = round(14 * FACTOR)            # pointer square, 14 points
BACKGROUND = (0xF4, 0xF2, 0xEE)
POINTER = (0x11, 0x75, 0x6B)
results = []
ok = True

def report(line, good):
    global ok
    results.append(line + (" ok" if good else " WRONG"))
    ok = ok and good

def near(px, colour, tol=8):
    return all(abs(a - b) <= tol for a, b in zip(px, colour))

def pixels_of(img, colour):
    w, h, p = img
    return [(i // 3 % w, i // 3 // w) for i in range(0, w * h * 3, 3) if near(p[i:i + 3], colour)]

def check_picture(name, img, pointer_at=None):
    """Background over at least 90% of the display; if pointer_at (absolute x, y) is given, the
    pointer square drawn there (its top-left corner at the pointer position, within 6 pixels)."""
    if img is None:
        report(f"PICTURE: {name}: no screenshot", False); return
    w, h, _ = img
    bg = len(pixels_of(img, BACKGROUND)) / (w * h)
    line, good = f"PICTURE: {name}: background {bg:.1%} of {w}x{h}", bg >= 0.9
    if pointer_at:
        ex, ey = pointer_at[0] / 32767 * w, pointer_at[1] / 32767 * h
        pts = pixels_of(img, POINTER)
        if pts:
            x0, y0 = min(p[0] for p in pts), min(p[1] for p in pts)
            good = good and abs(x0 - ex) <= 6 and abs(y0 - ey) <= 6 and len(pts) >= POINTER_PX ** 2 // 2
            line += f", pointer {len(pts)} pixels at ({x0}, {y0}), expected ({ex:.0f}, {ey:.0f})"
        else:
            good, line = False, line + f", no pointer, expected at ({ex:.0f}, {ey:.0f})"
    report(line, good)

def shot(q, name):
    ppm = os.path.abspath(os.path.join(res, name + ".ppm"))
    q.cmd("screendump", filename=ppm)
    time.sleep(1)
    if os.path.exists(ppm):
        img = ppm_to_png(ppm, ppm[:-4] + ".png")
        print("screenshot", name + ".png")
        return img
    return None

def move(q, x, y):
    q.cmd("input-send-event", events=[{"type": "abs", "data": {"axis": "x", "value": x}},
                                      {"type": "abs", "data": {"axis": "y", "value": y}}])

def check_input(kind, pattern, ax, ay):
    """The compositor's logged position (points) for this input, within 2 points of the target."""
    ex, ey = ax / 32767 * PTS_W, ay / 32767 * PTS_H
    with open(log_path, errors="replace") as f:
        m = re.search(pattern + r" at \((-?\d+), (-?\d+)\)", f.read())
    if not m:
        report(f"INPUT: {kind}: not reported, expected ({ex:.0f}, {ey:.0f})", False); return
    x, y = int(m.group(1)), int(m.group(2))
    report(f"INPUT: {kind} at ({x}, {y}), expected ({ex:.0f}, {ey:.0f})",
           abs(x - ex) <= 2 and abs(y - ey) <= 2)

if not wait_log("UNIT-DISPLAY: ready", 600):
    print("compositor not ready"); sys.exit(1)
q = Qmp(qmp_path)
time.sleep(3)
# 1. Start-up picture, before any input.
check_picture("1-startup", shot(q, "1-startup"))
# 2 and 3. Pointer at two distinct positions; each screenshot is taken after the frames that draw it.
A, B = (8192, 8192), (24576, 20480)
move(q, *A); time.sleep(1.5)
check_picture("2-pointer-a", shot(q, "2-pointer-a"), A)
move(q, *B); time.sleep(1.5)
check_picture("3-pointer-b", shot(q, "3-pointer-b"), B)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
time.sleep(0.2)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
# 4. Touch, independent of the pointer, at distinct x and y targets. QEMU's virtio input sends the
# slot and tracking ID for 'begin' and 'end' and the axis value for 'data': one batch of begin,
# x data and y data, then end with tracking ID -1.
T = (6000, 26000)
mtt = lambda kind, axis, value, tid=1: {"type": "mtt", "data": {"type": kind, "slot": 0,
                                         "tracking-id": tid, "axis": axis, "value": value}}
q.cmd("input-send-event", events=[mtt("begin", "x", T[0]), mtt("data", "x", T[0]), mtt("data", "y", T[1])])
time.sleep(0.3)
q.cmd("input-send-event", events=[mtt("end", "x", T[0], -1)])
time.sleep(1.5)
check_picture("4-after-touch", shot(q, "4-after-touch"))
if wait_log("UNIT-DISPLAY: PASS", 60):
    check_input("pointer press", r"input: pointer press", *B)
    check_input("touch down", r"input: touch down", *T)
else:
    report("INPUT: compositor did not report the input (no UNIT-DISPLAY: PASS)", False)
results.append("CHECK: PASS" if ok else "CHECK: FAIL")
with open(os.path.join(res, "check.txt"), "w") as f:
    f.write("\n".join(results) + "\n")
print("\n".join(results))
