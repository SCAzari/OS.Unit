#!/usr/bin/env python3
"""Host side of the display check: waits for 'UNIT-DISPLAY: ready' on the serial log, then takes
screenshots and sends pointer and touch input to the emulated machine through QMP."""
import json, os, socket, struct, sys, time, zlib

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

# Picture check: the screenshots must show the Unit background over most of the display and the
# pointer where it was moved; log lines alone cannot show that the right picture reached the screen.
BACKGROUND = (0xF4, 0xF2, 0xEE)
POINTER = (0x11, 0x75, 0x6B)
picture = []

def near(px, colour, tol=8):
    return all(abs(a - b) <= tol for a, b in zip(px, colour))

def count(img, colour):
    w, h, p = img
    return sum(1 for i in range(0, w * h * 3, 3) if near(p[i:i + 3], colour))

def check_picture(name, img, pointer):
    if img is None:
        picture.append(f"PICTURE: {name}: no screenshot"); return False
    w, h, _ = img
    bg = count(img, BACKGROUND) / (w * h)
    ok = bg >= 0.9
    line = f"PICTURE: {name}: background {bg:.1%} of {w}x{h}"
    if pointer:
        n = count(img, POINTER)
        ok = ok and n > 0
        line += f", pointer {n} pixels"
    picture.append(line + (" ok" if ok else " WRONG"))
    return ok

def shot(q, name):
    ppm = os.path.abspath(os.path.join(res, name + ".ppm"))
    q.cmd("screendump", filename=ppm)
    time.sleep(1)
    if os.path.exists(ppm):
        img = ppm_to_png(ppm, ppm[:-4] + ".png")
        print("screenshot", name + ".png")
        return img
    return None

if not wait_log("UNIT-DISPLAY: ready", 600):
    print("compositor not ready"); sys.exit(1)
q = Qmp(qmp_path)
time.sleep(3)
ok = check_picture("1-compositor-started", shot(q, "1-compositor-started"), True)
# Pointer: move to the middle of the display and click (absolute range 0..32767).
q.cmd("input-send-event", events=[{"type": "abs", "data": {"axis": "x", "value": 16384}},
                                  {"type": "abs", "data": {"axis": "y", "value": 16384}}])
time.sleep(0.5)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
time.sleep(0.2)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
time.sleep(2)
ok = check_picture("2-pointer-moved", shot(q, "2-pointer-moved"), True) and ok
# Touch: one finger down and up at a quarter of the display.
q.cmd("input-send-event", events=[
    {"type": "mtt", "data": {"type": "begin", "slot": 0, "tracking-id": 1, "axis": "x", "value": 8192}},
    {"type": "mtt", "data": {"type": "begin", "slot": 0, "tracking-id": 1, "axis": "y", "value": 8192}}])
time.sleep(0.3)
q.cmd("input-send-event", events=[
    {"type": "mtt", "data": {"type": "end", "slot": 0, "tracking-id": 1, "axis": "x", "value": 8192}}])
time.sleep(2)
ok = check_picture("3-after-touch", shot(q, "3-after-touch"), False) and ok
picture.append("PICTURE: PASS" if ok else "PICTURE: FAIL")
with open(os.path.join(res, "picture.txt"), "w") as f:
    f.write("\n".join(picture) + "\n")
print("\n".join(picture))
wait_log("UNIT-DISPLAY: PASS", 60)
