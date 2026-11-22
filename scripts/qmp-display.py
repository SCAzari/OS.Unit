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

def shot(q, name):
    ppm = os.path.abspath(os.path.join(res, name + ".ppm"))
    q.cmd("screendump", filename=ppm)
    time.sleep(1)
    if os.path.exists(ppm):
        ppm_to_png(ppm, ppm[:-4] + ".png")
        print("screenshot", name + ".png")

if not wait_log("UNIT-DISPLAY: ready", 600):
    print("compositor not ready"); sys.exit(1)
q = Qmp(qmp_path)
time.sleep(3)
shot(q, "1-compositor-started")
# Pointer: move to the middle of the display and click (absolute range 0..32767).
q.cmd("input-send-event", events=[{"type": "abs", "data": {"axis": "x", "value": 16384}},
                                  {"type": "abs", "data": {"axis": "y", "value": 16384}}])
time.sleep(0.5)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
time.sleep(0.2)
q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
time.sleep(2)
shot(q, "2-pointer-moved")
# Touch: one finger down and up at a quarter of the display.
q.cmd("input-send-event", events=[
    {"type": "mtt", "data": {"type": "begin", "slot": 0, "tracking-id": 1, "axis": "x", "value": 8192}},
    {"type": "mtt", "data": {"type": "begin", "slot": 0, "tracking-id": 1, "axis": "y", "value": 8192}}])
time.sleep(0.3)
q.cmd("input-send-event", events=[
    {"type": "mtt", "data": {"type": "end", "slot": 0, "tracking-id": 1, "axis": "x", "value": 8192}}])
wait_log("UNIT-DISPLAY: PASS", 60)
