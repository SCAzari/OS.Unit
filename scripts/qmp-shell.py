#!/usr/bin/env python3
"""Host side of the Shell UI check (UNIT-OS-WP-0009): waits for 'UNIT-SHELL: ready' on the serial
log, lets the shell settle, takes a screenshot through QMP and checks that the home screen's parts
were drawn in the Daylight light colours (Visual Design Specification §3.1): the status bar's time
and icons, the large clock, and the navigation bar's three glyphs. Writes PICTURE: lines and a
final CHECK: PASS or CHECK: FAIL to <results>/check.txt.

The QMP and PNG helpers repeat those of qmp-display.py on purpose, so the display check, which
is the compositor's acceptance evidence, is not changed by this check."""
import json, os, socket, struct, sys, time, zlib

qmp_path, log_path, res = sys.argv[1:4]
SETTLE = float(os.environ.get("SHELL_SETTLE", "10"))  # Qt draws slowly in software emulation

def wait_log(text, timeout):
    end = time.time() + timeout
    while time.time() < end:
        try:
            with open(log_path, errors="replace") as f:
                data = f.read()
            if text in data:
                return True
            if "UNIT-SHELL: FAIL" in data or "UNIT-SHELL: PASS" in data:
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

# 720 x 1560 pixels at 720 / 412 pixels per point, as in qmp-display.py.
W, H, FACTOR = 720, 1560, 720 / 412
BACKGROUND = (0xF4, 0xF2, 0xEE)
TEXT = (0x1E, 0x26, 0x26)
SECONDARY = (0x56, 0x63, 0x62)

results, ok = [], True

def report(line, good):
    global ok
    results.append(line + (" ok" if good else " WRONG"))
    ok = ok and good

def near(px, colour, tol):
    return all(abs(a - b) <= tol for a, b in zip(px, colour))

def count(img, colour, box, tol=24):
    """Pixels close to `colour` inside box = (x0, y0, x1, y1) in points."""
    w, h, p = img
    x0, y0, x1, y1 = (round(v * FACTOR) for v in box)
    n = 0
    for y in range(max(0, y0), min(h, y1)):
        row = y * w * 3
        for x in range(max(0, x0), min(w, x1)):
            i = row + x * 3
            if near(p[i:i + 3], colour, tol):
                n += 1
    return n

def check(img):
    w, h, _ = img
    pts_h = h / FACTOR
    bg = count(img, BACKGROUND, (0, 0, w / FACTOR, pts_h), tol=8) / (w * h)
    report(f"PICTURE: home: background colour on {bg:.1%} of the display (at least 60%)", bg >= 0.6)
    n = count(img, SECONDARY, (0, 0, 412, 36))
    report(f"PICTURE: status bar: {n} pixels in the secondary text colour (time, icons; at least 150)", n >= 150)
    n = count(img, TEXT, (16, 50, 260, 140))
    report(f"PICTURE: clock: {n} pixels in the text colour (at least 1500)", n >= 1500)
    n = count(img, TEXT, (96, pts_h - 72, 316, pts_h - 12))
    report(f"PICTURE: navigation bar: {n} pixels of glyphs in the text colour (at least 100)", n >= 100)

def main():
    q = Qmp(qmp_path)
    if not wait_log("UNIT-SHELL: ready", 600):
        report("PICTURE: the shell did not report ready", False)
    else:
        time.sleep(SETTLE)
        ppm = os.path.abspath(os.path.join(res, "1-home.ppm"))
        q.cmd("screendump", filename=ppm)
        time.sleep(1)
        if os.path.exists(ppm):
            check(ppm_to_png(ppm, ppm[:-4] + ".png"))
            print("screenshot 1-home.png")
        else:
            report("PICTURE: no screenshot", False)
    with open(os.path.join(res, "check.txt"), "w") as f:
        f.write("\n".join(results + ["CHECK: " + ("PASS" if ok else "FAIL")]) + "\n")

main()
