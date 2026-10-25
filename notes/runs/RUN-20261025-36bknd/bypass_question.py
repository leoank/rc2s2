"""RUN-20261025-36bknd: a bypass-permissions session under the wrapper asked to pose a question; which hooks
fire for AskUserQuestion, whether the node holds it, and the dialog answered at the terminal.
Terminal and hub helpers as in RUN-20261025-2aymst's driver.
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

def hook_kinds(sid, after):
    return q("SELECT kind, payload FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
             "AND seq > ? ORDER BY seq", sid, after)

t = Term(["--dangerously-skip-permissions"])
try:
    t.ready()
    sid = wrapped_session(t)
    say("session ready under the wrapper in bypass mode")
    mark = last_seq(sid)
    t.typed("Use the AskUserQuestion tool to ask me one question: which colour do I prefer, "
            "with the options red and blue. Then reply with just the colour I chose.")
    # The dialog at the terminal, or a hold: wait for the tool call to show up either way.
    t.until("the question at the terminal or held", lambda: b"colour" in t.screen[-4000:] and
            (b"Red" in t.screen[-4000:] or b"red" in t.screen[-4000:]) or
            q("SELECT id FROM pending WHERE session_id = ? AND closed_ms IS NULL", sid), 60)
    t.sleep(2)
    kinds = [(k, json.loads(p).get("tool_name"), json.loads(p).get("permission_mode")) for k, p in hook_kinds(sid, mark)]
    pend = q("SELECT tool_name, kind, reason FROM pending WHERE session_id = ?", sid)
    say(f"hooks for the question so far: {kinds}")
    say(f"pending rows: {pend}")
    say(f"dialog at the terminal: {b'Red' in t.screen[-4000:] or b'red' in t.screen[-4000:]}")
    # Answer at the terminal: Enter picks the highlighted option.
    os.write(t.fd, b"\r")
    up_seq = mark
    t.until("Stop after the answer", lambda: hooks(sid, "Stop", up_seq), 90)
    t.sleep(2)
    kinds = [(k, json.loads(p).get("tool_name")) for k, p in hook_kinds(sid, mark)]
    say(f"hooks for the whole turn: {kinds}")
    say(f"reply {reply(t, sid, mark)!r}")
except SystemExit as e:
    say(f"{e}")
    raise
finally:
    t.stop()
