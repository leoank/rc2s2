"""RUN-20261025-2g2812: one wrapped session with turns of different shapes, then `/cost` at the
terminal; compares the transcript's turn_duration lines and the daemon's usage totals with
what Claude Code reports. Terminal and hub helpers as in RUN-20261025-2aymst's driver.
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

def usage_totals(sid):
    rows = q("SELECT payload FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_DAEMON' "
             "AND kind = 'usage' ORDER BY seq DESC LIMIT 1", sid)
    return json.loads(rows[0][0]) if rows else {}

def turn_durations(sid):
    out = []
    for (seq, payload, t) in q("SELECT seq, payload, time_ms FROM events WHERE session_id = ? AND kind = 'system' "
                               "AND source = 'EVENT_SOURCE_TRANSCRIPT' ORDER BY seq", sid):
        v = json.loads(payload)
        if v.get("subtype") == "turn_duration":
            out.append((seq, v.get("durationMs"), t))
    return out

def hook_times(sid, kind):
    return q("SELECT seq, time_ms FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
             "AND kind = ? ORDER BY seq", sid, kind)

def pending_open(sid):
    return q("SELECT id FROM pending WHERE session_id = ? AND closed_ms IS NULL", sid)

def fmt_k(n):
    return f"{n/1000:.1f}k" if n >= 1000 else str(n)

t = Term([])
try:
    t.ready()
    sid = wrapped_session(t)
    node = node_id()
    say("session ready under the wrapper")

    def turn(text, allow=False, timeout=120):
        mark = last_seq(sid)
        t.typed(text)
        up = t.until("UserPromptSubmit", lambda: hooks(sid, "UserPromptSubmit", mark), 60)
        if allow:
            # A command may be held (then allowed from here) or run without a dialog.
            held = t.until("a held request or the turn's end",
                           lambda: pending_open(sid) or hooks(sid, "Stop", up[0][0]), 120)
            if held and isinstance(held[0][0], str) and not hooks(sid, "Stop", up[0][0]):
                ctl("allow", node, held[0][0])
        t.until("Stop", lambda: hooks(sid, "Stop", up[0][0]), timeout)
        t.sleep(2)
        return mark

    turn("Reply with just the word pelican.")
    turn("Run exactly this one shell command: ls -la", allow=True)
    turn("Write three short sentences about herons, no tools.")
    # An interrupted turn: no Stop fires for it; the follow-up is a turn of its own.
    mark = last_seq(sid)
    t.typed("Count from 1 to 3000, one number per line, nothing else, no tools.")
    up_long = t.until("the long turn", lambda: hooks(sid, "UserPromptSubmit", mark), 60)
    t.sleep(3)
    ctl("interrupt", node, sid, "--then", "Reply with just the word osprey.")
    up = t.until("the follow-up", lambda: hooks(sid, "UserPromptSubmit", up_long[0][0]), 60)
    t.until("Stop after the follow-up", lambda: hooks(sid, "Stop", up[0][0]), 120)
    t.sleep(2)
    turn("Use the Agent tool once with a short prompt asking the agent to reply with the word kestrel, then tell me what it said.", timeout=300)

    # /cost at the terminal, read from the screen.
    before = len(t.screen)
    t.typed("/cost")
    t.until("the cost report", lambda: b"Usage by model" in t.screen[before:] or b"Total cost" in t.screen[before:], 30)
    t.sleep(2)
    shown = t.screen[before:].decode(errors="replace")
    cost_lines = [l.strip() for l in shown.splitlines() if re.search(r"Total (cost|duration)|Usage by model|input|output|cache", l)]
    say("cost report: " + " | ".join(dict.fromkeys(cost_lines)))

    # The hub's record against it.
    durs = turn_durations(sid)
    ups = hook_times(sid, "UserPromptSubmit")
    stops = hook_times(sid, "Stop")
    say(f"turns: UserPromptSubmit {len(ups)}, Stop {len(stops)}, turn_duration {len(durs)}: "
        + ", ".join(f"{d} ms" for _, d, _ in durs))
    # Each Stop against the turn_duration that follows it and the UserPromptSubmit before it.
    pairs = []
    for s_seq, s_t in stops:
        u = max((u_t for u_seq, u_t in ups if u_seq < s_seq), default=None)
        d = next((d for seq, d, _ in durs if seq > s_seq), None)
        if u is not None and d is not None:
            pairs.append((s_t - u, d))
    say("hook span vs turn_duration (ms): " + ", ".join(f"{a} vs {b}" for a, b in pairs))
    totals = usage_totals(sid)
    say("daemon totals by model: " + "; ".join(
        f"{m}: {fmt_k(v['input_tokens'])} input, {fmt_k(v['output_tokens'])} output, "
        f"{fmt_k(v['cache_read_input_tokens'])} cache read, {fmt_k(v['cache_creation_input_tokens'])} cache write, "
        f"{v['messages']} messages" for m, v in totals.items()))
    ctl("kill", node, sid)
except SystemExit as e:
    say(f"{e}")
    raise
finally:
    t.stop()
