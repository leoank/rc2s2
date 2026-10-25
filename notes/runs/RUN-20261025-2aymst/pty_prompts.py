"""RUN-20261025-2aymst: plays the person at an `rc2s2 run` terminal and a client sending
prompts through `rc2s2 ctl`, then summarises the hub's record of these sessions only.
Terminal helpers as in RUN-20261019-mvz0r3's driver. Prints no content from any other
session."""
import argparse, fcntl, json, os, pty, re, select, signal, sqlite3, struct, subprocess, termios, time

ap = argparse.ArgumentParser()
for a in ["bin", "socket", "settings", "work", "db", "claude", "log", "token-file"]:
    ap.add_argument("--" + a, required=True)
A = ap.parse_args()
ANSI = re.compile(rb"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)|\x1bP[^\x1b]*\x1b\\|\x1b[()][A-Z0-9]|\x1b[=>]")
HUB = "http://127.0.0.1:7700"
log = open(A.log, "ab")

# --- the terminal ------------------------------------------------------------------------
class Term:
    def __init__(self, extra):
        env = {k: v for k, v in os.environ.items() if not (k.startswith("CLAUDE") or k == "AI_AGENT")}
        env["TERM"] = "xterm-256color"
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(A.work)
            os.execve(f"{A.bin}/rc2s2", ["rc2s2", "run", "--command", A.claude, "--wrapper-socket", A.socket,
                                         "--", "--settings", A.settings, "--model", "haiku", *extra], env)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        self.screen = b""

    def pump(self, timeout=0.2):
        r, _, _ = select.select([self.fd], [], [], timeout)
        if self.fd in r:
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                return False
            if not data:
                return False
            log.write(data); log.flush()
            self.screen += ANSI.sub(b"", data)
        return True

    def sleep(self, s):
        end = time.time() + s
        while time.time() < end:
            self.pump(0.1)

    def typed(self, text):
        os.write(self.fd, text.encode()); self.sleep(1); os.write(self.fd, b"\r")

    def until(self, what, fn, timeout=120):
        end = time.time() + timeout
        while time.time() < end:
            v = fn()
            if v:
                return v
            self.pump(0.2)
        raise SystemExit(f"timed out waiting for {what}")

    def ready(self):
        # The prompt box, or the one-time bypass warning, which Enter accepts.
        self.until("the prompt box", lambda: b"/effort" in self.screen or b"shortcuts" in self.screen
                   or b"Yes, I accept" in self.screen, 90)
        if b"Yes, I accept" in self.screen and b"shortcuts" not in self.screen:
            os.write(self.fd, b"\r")
            self.until("the prompt box after the warning", lambda: b"/effort" in self.screen or b"shortcuts" in self.screen, 60)
        self.sleep(2)

    def stop(self):
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        end = time.time() + 10
        while time.time() < end and not os.waitpid(self.pid, os.WNOHANG)[0]:
            self.pump(0.2)

# --- the hub's record ----------------------------------------------------------------------
def q(sql, *a):
    with sqlite3.connect(f"file:{A.db}?mode=ro", uri=True) as c:
        return c.execute(sql, a).fetchall()

def hooks(sid, kind, after=0):
    return q("SELECT seq, payload FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
             "AND kind = ? AND seq > ? ORDER BY seq", sid, kind, after)

def daemon(sid, kind, after=0):
    return q("SELECT seq FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_DAEMON' "
             "AND kind = ? AND seq > ?", sid, kind, after)

def last_seq(sid):
    return q("SELECT COALESCE(MAX(seq), 0) FROM events WHERE session_id = ?", sid)[0][0]

def user_lines(sid, after):
    out = []
    for (payload,) in q("SELECT payload FROM events WHERE session_id = ? AND kind = 'user' "
                        "AND source = 'EVENT_SOURCE_TRANSCRIPT' AND seq > ? ORDER BY seq", sid, after):
        v = json.loads(payload)
        c = v.get("message", {}).get("content")
        if isinstance(c, str):
            out.append(c)
        elif isinstance(c, list):
            out.extend(b.get("text", "") for b in c if b.get("type") == "text")
    return out

def _reply(sid, after):
    texts = []
    for (payload,) in q("SELECT payload FROM events WHERE session_id = ? AND kind = 'assistant' "
                        "AND source = 'EVENT_SOURCE_TRANSCRIPT' AND seq > ? ORDER BY seq", sid, after):
        for block in json.loads(payload).get("message", {}).get("content", []):
            if isinstance(block, dict) and block.get("type") == "text":
                texts.append(block["text"].strip())
    return texts[-1] if texts else ""

def reply(term, sid, after):
    # The daemon reads the transcript every 500 ms, so the text lands after the Stop hook.
    # Clipped: a reply can be the long count of step 2.
    text = term.until("the reply in the transcript", lambda: _reply(sid, after), 15)
    return text if len(text) <= 60 else text[:60] + "…"

def wrapped_session(term):
    return term.until("the session wrapped", lambda: q(
        "SELECT id FROM sessions WHERE launch = 'LAUNCH_WRAPPER' AND ended_ms IS NULL"), 30)[0][0]

# --- the client ------------------------------------------------------------------------------
def ctl(*args):
    r = subprocess.run([f"{A.bin}/rc2s2", "ctl", "--hub", HUB, "--token-file", A.token_file, *args],
                       capture_output=True, text=True)
    return (r.stdout + r.stderr).strip()

def node_id():
    return q("SELECT id FROM nodes LIMIT 1")[0][0]

out = []
def say(line):
    out.append(line); print(line, flush=True)

t = Term([])
try:
    t.ready()
    sid = wrapped_session(t)
    node = node_id()
    say(f"session 1 ready under the wrapper")

    # 1. Idle: the prompt typed into the PTY by the daemon.
    mark = last_seq(sid)
    note = ctl("prompt", node, sid, "Reply with just the word pelican.")
    up = t.until("UserPromptSubmit for the idle prompt", lambda: hooks(sid, "UserPromptSubmit", mark), 60)
    t.until("Stop after it", lambda: hooks(sid, "Stop", up[0][0]), 90)
    lines = user_lines(sid, mark)
    say(f"1 idle: ctl said {note!r}; UserPromptSubmit prompt {json.loads(up[0][1]).get('prompt')!r}; "
        f"user lines {lines!r}; banner {any('Another Claude session' in l for l in lines)}; "
        f"reply {reply(t, sid, mark)!r}")

    # 2. Busy: a long turn at the terminal, a prompt from the client 3 s in.
    mark = last_seq(sid)
    t.typed("Count from 1 to 3000, one number per line, nothing else, no tools.")
    up_long = t.until("the long turn to start", lambda: hooks(sid, "UserPromptSubmit", mark))
    t.sleep(3)
    running = not hooks(sid, "Stop", up_long[0][0])
    mark2 = last_seq(sid)
    sent_at = time.time()
    note = ctl("prompt", node, sid, "Reply with just the word heron.")
    up = t.until("the follow-up's UserPromptSubmit", lambda: hooks(sid, "UserPromptSubmit", mark2), 300)
    arrived = time.time() - sent_at
    stops_before = [s for s, _ in hooks(sid, "Stop", up_long[0][0]) if s < up[0][0]]
    t.until("Stop after the follow-up", lambda: hooks(sid, "Stop", up[0][0]), 120)
    say(f"2 busy: turn running {running}; ctl said {note!r}; follow-up arrived after {arrived:.1f} s; "
        f"Stop hooks before it {len(stops_before)}; interrupted events {len(daemon(sid, 'interrupted', mark))}; "
        f"follow-up prompt {json.loads(up[0][1]).get('prompt')!r}; reply {reply(t, sid, up[0][0])!r}; "
        f"count reached {t.screen.count(b'2999') > 0 or b'3000' in t.screen}")
    ctl("kill", node, sid)
    t.stop()

    # 3. Bypass mode: a prompt must not be held (ADR-0022 was about the socket).
    t = Term(["--dangerously-skip-permissions"])
    t.ready()
    sid2 = t.until("session 2 wrapped", lambda: [r for r in q(
        "SELECT id FROM sessions WHERE launch = 'LAUNCH_WRAPPER' AND ended_ms IS NULL") if r[0] != sid], 30)[0][0]
    mark = last_seq(sid2)
    note = ctl("prompt", node, sid2, "Reply with just the word osprey.")
    up = t.until("UserPromptSubmit in bypass mode", lambda: hooks(sid2, "UserPromptSubmit", mark), 60)
    t.until("Stop after it", lambda: hooks(sid2, "Stop", up[0][0]), 90)
    say(f"3 bypass: ctl said {note!r}; permission_mode {json.loads(up[0][1]).get('permission_mode')!r}; "
        f"held at the terminal {b'approve' in t.screen.lower() and b'peer' in t.screen.lower()}; "
        f"reply {reply(t, sid2, mark)!r}")
    ctl("kill", node, sid2)
except SystemExit as e:
    say(f"{e}")
    raise
finally:
    t.stop()
