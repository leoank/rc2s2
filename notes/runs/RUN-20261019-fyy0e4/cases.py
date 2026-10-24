"""RUN-20261019-fyy0e4 and RUN-20261019-fzgrw0 (from the RUN-20261019-e78rne driver), with
--case bypass (a prompt to a session that bypasses permissions) or --case keys (held
requests answered at the terminal with Escape and with 2): plays the person at an `rc2s2 run` terminal and a client through
`rc2s2 ctl`, then summarises the hub's record of this one session. Prints no content from
any other session; the terminal output stays in --log."""
import argparse, json, os, pty, re, select, signal, sqlite3, subprocess, sys, time

ap = argparse.ArgumentParser()
for a in ["bin", "hub", "client-token", "socket", "settings", "work", "out", "db", "claude", "log"]:
    ap.add_argument("--" + a, required=True)
ap.add_argument("--case", choices=["bypass", "keys"], required=True)
A = ap.parse_args()
ANSI = re.compile(rb"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)|\x1bP[^\x1b]*\x1b\\|\x1b[()][A-Z0-9]|\x1b[=>]")

env = {k: v for k, v in os.environ.items() if not (k.startswith("CLAUDE") or k == "AI_AGENT")}
env["TERM"] = "xterm-256color"
pid, fd = pty.fork()
if pid == 0:
    os.chdir(A.work)
    os.execve(f"{A.bin}/rc2s2", ["rc2s2", "run", "--command", A.claude, "--wrapper-socket", A.socket,
                                 "--", "--settings", A.settings, "--model", "haiku",
                                 *(["--dangerously-skip-permissions"] if A.case == "bypass" else [])], env)
import fcntl, struct, termios
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
log = open(A.log, "ab")
screen = b""
t0 = time.time()

def pump(timeout=0.2):
    global screen
    r, _, _ = select.select([fd], [], [], timeout)
    if fd in r:
        try:
            data = os.read(fd, 65536)
        except OSError:
            return False
        if not data:
            return False
        log.write(data); log.flush()
        screen += ANSI.sub(b"", data)
    return True

def sleep(s):
    end = time.time() + s
    while time.time() < end:
        pump(0.1)

def typed(text):
    os.write(fd, text.encode()); sleep(1); os.write(fd, b"\r")

def db():
    return sqlite3.connect(f"file:{A.db}?mode=ro", uri=True)

def ctl(*args):
    p = subprocess.run([f"{A.bin}/rc2s2", "ctl", "--hub", A.hub, "--token-file", A.client_token, *args],
                       capture_output=True, text=True)
    return (p.stdout + p.stderr).strip()

def until(what, fn, timeout=120):
    end = time.time() + timeout
    while time.time() < end:
        v = fn()
        if v:
            return v
        pump(0.2)
    raise SystemExit(f"timed out waiting for {what}")

def q(sql, *a):
    with db() as c:
        return c.execute(sql, a).fetchall()

def hooks(sid, kind, after=0):
    return q("SELECT seq, payload FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
             "AND kind = ? AND seq > ? ORDER BY seq", sid, kind, after)

def last_seq(sid):
    return q("SELECT COALESCE(MAX(seq), 0) FROM events WHERE session_id = ?", sid)[0][0]

def reply(sid, after):
    # The daemon reads the transcript every 500 ms, so the text lands after the Stop hook.
    return until("the reply in the transcript", lambda: _reply(sid, after), 15)

def _reply(sid, after):
    texts = []
    for (payload,) in q("SELECT payload FROM events WHERE session_id = ? AND kind = 'assistant' "
                        "AND source = 'EVENT_SOURCE_TRANSCRIPT' AND seq > ? ORDER BY seq", sid, after):
        for block in json.loads(payload).get("message", {}).get("content", []):
            if isinstance(block, dict) and block.get("type") == "text":
                texts.append(block["text"].strip())
    return texts[-1] if texts else ""

def open_pending(sid, tool):
    return q("SELECT id, tool_input, created_ms FROM pending WHERE session_id = ? AND tool_name = ? "
             "AND closed_ms IS NULL", sid, tool)

def closed(pid_):
    return q("SELECT reason, closed_ms, resolution FROM pending WHERE id = ? AND closed_ms IS NOT NULL", pid_)

out = []
def say(line):
    out.append(line); print(line, flush=True)

try:
    if A.case == "bypass":
        # Claude Code asks once whether to accept bypass mode; phase 0 accepted it already,
        # and the answer is the same one: Down to "Yes, I accept", then Enter.
        end = time.time() + 15
        # The TUI positions words with cursor moves, so the stripped text has no spaces.
        while time.time() < end and b"Iaccept" not in screen and b"/effort" not in screen:
            pump(0.2)
        if b"Iaccept" in screen:
            os.write(fd, b"\x1b[B"); sleep(1); os.write(fd, b"\r")
            say("bypass warning shown and accepted")
    until("the prompt box", lambda: b"/effort" in screen or b"shortcuts" in screen, 90)
    say(f"session ready under the wrapper after {time.time() - t0:.1f} s")
    sleep(2)
    sid = until("the session wrapped", lambda: q(
        "SELECT id FROM sessions WHERE launch = 'LAUNCH_WRAPPER'"), 30)[0][0]

    if A.case == "bypass":
        mark = last_seq(sid)
        typed("Reply with just the word ok.")
        until("Stop after the first turn", lambda: hooks(sid, "Stop", mark))
        mode = json.loads(hooks(sid, "UserPromptSubmit", mark)[0][1]).get("permission_mode")
        say(f"1 typed turn: hook permission_mode {mode!r}; reply {reply(sid, mark)!r}")
        mark = last_seq(sid)
        seen = len(screen)
        say(f"2 ctl prompt: {ctl('prompt', 'run-node', sid, 'Reply with just the word pelican.')}")
        sleep(20)
        held = b"Held peer message" in screen[seen:]
        say(f"  after 20 s: UserPromptSubmit {bool(hooks(sid, 'UserPromptSubmit', mark))}, "
            f"Stop {bool(hooks(sid, 'Stop', mark))}, terminal shows a held peer message {held}")

    if A.case == "keys":
        for n, key, name in [(1, b"\x1b", "Escape"), (2, b"2", "2")]:
            target = os.path.join(A.out, f"key-{n}.txt")
            mark = last_seq(sid)
            typed(f"Run exactly this one shell command and nothing else: touch {target}")
            p_id, _, _ = until("the command held", lambda: open_pending(sid, "Bash"))[0]
            sleep(3)
            sent_ms = int(time.time() * 1000)
            os.write(fd, key)
            reason, closed_ms, _ = until("the hold released", lambda: closed(p_id), 30)[0]
            sleep(8)  # let the dialog's answer and the end of the turn land
            stop = bool(hooks(sid, "Stop", mark))
            post = bool(hooks(sid, "PostToolUse", mark))
            say(f"{n} {name} at the terminal: closed {reason} {closed_ms - sent_ms} ms after the key; "
                f"PostToolUse {post}; Stop {stop}; file exists {os.path.exists(target)}")

    say(f"ctl kill: {ctl('kill', 'run-node', sid)}")
    start = time.time()
    while time.time() - start < 15:
        pump(0.2)
        if os.waitpid(pid, os.WNOHANG)[0]:
            break
    say(f"  wrapper exited after {time.time() - start:.1f} s")
    pend = q("SELECT tool_name, kind, reason FROM pending WHERE session_id = ? ORDER BY created_ms", sid)
    say("pending requests: " + ("; ".join(f"{t} {k} {r}" for t, k, r in pend) or "none"))
    hk = q("SELECT kind, COUNT(*) FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
           "GROUP BY kind ORDER BY kind", sid)
    say("hook events: " + ", ".join(f"{k} {n}" for k, n in hk))
except SystemExit as e:
    # What this session's terminal showed last, to see why a step never came.
    tail = re.sub(rb"\s+", b" ", screen[-1500:]).decode("utf-8", "replace")
    say(f"{e}; the terminal showed last: {tail}")
    raise
finally:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
