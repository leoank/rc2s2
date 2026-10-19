#!/usr/bin/env python3
"""Run a command under a PTY and drive it with a scripted sequence.

Usage: pty_driver.py OUTLOG STEPS_JSON -- cmd args...
Steps: [{"wait": "regex", "timeout": 30}, {"send": "text"}, {"sleep": 2},
        {"wait_file_contains": [path, "substr"], "timeout": 60}, {"write_file": [path, "content"]}]
All output is appended to OUTLOG (raw) and OUTLOG.txt (ANSI stripped).
"""
import fcntl, json, os, pty, re, select, struct, sys, termios, time, signal

outlog, steps = sys.argv[1], json.loads(sys.argv[2])
cmd = sys.argv[sys.argv.index("--") + 1:]
pid, fd = pty.fork()
if pid == 0:
    for k in [k for k in os.environ if k.startswith("CLAUDE") or k == "AI_AGENT"]:
        del os.environ[k]
    os.environ["TERM"] = "xterm-256color"
    os.execvp(cmd[0], cmd)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
ansi = re.compile(rb"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07|\x1b[()][A-Z0-9]|\x1b[=>]|\r")
buf = b""
raw = open(outlog, "ab")
txt = open(outlog + ".txt", "ab")

def pump(timeout):
    global buf
    r, _, _ = select.select([fd], [], [], timeout)
    if fd in r:
        try:
            data = os.read(fd, 65536)
        except OSError:
            return False
        if not data:
            return False
        buf += data
        raw.write(data); raw.flush()
        txt.write(ansi.sub(b"", data)); txt.flush()
    return True

def note(s):
    line = f"\n### driver {time.strftime('%H:%M:%S')}: {s}\n".encode()
    raw.write(line); txt.write(line); raw.flush(); txt.flush()
    print(s, flush=True)

alive = True
for st in steps:
    if "wait" in st:
        pat = re.compile(st["wait"].encode())
        t0 = time.time(); start = len(buf)
        while time.time() - t0 < st.get("timeout", 30):
            if pat.search(ansi.sub(b"", buf[start:])):
                note(f"matched {st['wait']!r} after {time.time()-t0:.1f}s"); break
            if not pump(0.5): alive = False; break
        else:
            note(f"TIMEOUT waiting for {st['wait']!r}")
    elif "send" in st:
        pump(0); os.write(fd, st["send"].encode()); note(f"sent {st['send']!r}")
    elif "sleep" in st:
        t0 = time.time()
        while time.time() - t0 < st["sleep"]:
            if not pump(0.5): alive = False; break
        note(f"slept {st['sleep']}")
    elif "wait_file_contains" in st:
        path, sub = st["wait_file_contains"]; t0 = time.time()
        while time.time() - t0 < st.get("timeout", 60):
            if os.path.exists(path) and sub in open(path).read():
                note(f"file {path} contains {sub!r} after {time.time()-t0:.1f}s"); break
            if not pump(0.5): alive = False; break
        else:
            note(f"TIMEOUT waiting for {sub!r} in {path}")
    elif "write_file" in st:
        path, content = st["write_file"]
        open(path, "w").write(content); note(f"wrote {path}")
    if not alive:
        note("child exited"); break
# drain and finish
t0 = time.time()
while time.time() - t0 < 5 and pump(0.5):
    pass
try:
    os.kill(pid, signal.SIGTERM)
except ProcessLookupError:
    pass
note("done")
