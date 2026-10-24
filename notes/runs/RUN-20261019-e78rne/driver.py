"""RUN-20261019-e78rne (from the RUN-20261019-e63842 driver, with --steps, replies read once the
transcript has settled, and a turn long enough to interrupt): plays the person at an `rc2s2 run` terminal and a client through
`rc2s2 ctl`, then summarises the hub's record of this one session. Prints no content from
any other session; the terminal output stays in --log."""
import argparse, json, os, pty, re, select, signal, sqlite3, subprocess, sys, time

ap = argparse.ArgumentParser()
for a in ["bin", "hub", "client-token", "socket", "settings", "work", "out", "db", "claude", "log"]:
    ap.add_argument("--" + a, required=True)
ap.add_argument("--steps", default="1,2,3,4,5")
A = ap.parse_args()
ANSI = re.compile(rb"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)|\x1bP[^\x1b]*\x1b\\|\x1b[()][A-Z0-9]|\x1b[=>]")

env = {k: v for k, v in os.environ.items() if not (k.startswith("CLAUDE") or k == "AI_AGENT")}
env["TERM"] = "xterm-256color"
pid, fd = pty.fork()
if pid == 0:
    os.chdir(A.work)
    os.execve(f"{A.bin}/rc2s2", ["rc2s2", "run", "--command", A.claude, "--wrapper-socket", A.socket,
                                 "--", "--settings", A.settings, "--model", "haiku"], env)
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
    # The status line under the prompt box; its wording depends on the permission mode.
    until("the prompt box", lambda: b"/effort" in screen or b"shortcuts" in screen, 90)
    say(f"session ready under the wrapper after {time.time() - t0:.1f} s")
    steps = set(A.steps.split(","))
    # The registry names the session as soon as Claude Code starts; the wrapper marks it.
    sid = until("the session wrapped", lambda: q(
        "SELECT id FROM sessions WHERE launch = 'LAUNCH_WRAPPER'"), 30)[0][0]
    say("session recorded as LAUNCH_WRAPPER")

    if "1" in steps:
        # A question, answered from the client with free text.
        typed("Use the AskUserQuestion tool to ask me one question: which colour do I prefer, "
              "with the options red and blue. Then reply with just the colour I chose.")
        p_id, tool_input, _ = until("the question held", lambda: open_pending(sid, "AskUserQuestion"))[0]
        question = json.loads(tool_input)["questions"][0]["question"]
        mark = last_seq(sid)
        answer = ctl("answer", "run-node", p_id, "--answer", question + "=green, actually")
        say(f"1 question held: {question!r}; ctl answer: {answer}")
        until("Stop after the answer", lambda: hooks(sid, "Stop", mark))
        say(f"  closed {closed(p_id)[0][0]}; reply: {reply(sid, mark)!r}")

    if "2" in steps:
        # A command answered by Enter at the terminal while the request is held.
        target = os.path.join(A.out, "kept.txt")
        mark = last_seq(sid)
        typed(f"Run exactly this one shell command and nothing else: touch {target}")
        p_id, _, _ = until("the command held", lambda: open_pending(sid, "Bash"))[0]
        sleep(3)  # long enough to show the hold is not ending by itself
        sent_ms = int(time.time() * 1000)
        os.write(fd, b"\r")
        reason, closed_ms, _ = until("the hold released", lambda: closed(p_id), 30)[0]
        until("Stop after the command", lambda: hooks(sid, "Stop", mark))
        say(f"2 Enter at the terminal: closed {reason} {closed_ms - sent_ms} ms after the keystroke; "
            f"file exists {os.path.exists(target)}")

    if "3" in steps:
        # A prompt from the client over the messaging socket.
        mark = last_seq(sid)
        say(f"3 ctl prompt: {ctl('prompt', 'run-node', sid, 'Reply with just the word pelican.')}")
        until("Stop after the socket prompt", lambda: hooks(sid, "Stop", mark), 120)
        ups = hooks(sid, "UserPromptSubmit", mark)
        say(f"  UserPromptSubmit fired: {bool(ups)}; reply: {reply(sid, mark)!r}")

    if "4" in steps:
        # A turn long enough to be interrupted, then a follow-up typed after the interrupt.
        mark = last_seq(sid)
        typed("Count from 1 to 3000, one number per line, nothing else, no tools.")
        up_long = until("the long turn to start", lambda: hooks(sid, "UserPromptSubmit", mark))
        sleep(4)
        running = not hooks(sid, "Stop", up_long[0][0])
        mark2 = last_seq(sid)
        say(f"4 turn still running at the interrupt: {running}; "
            f"ctl interrupt: {ctl('interrupt', 'run-node', sid, '--then', 'Reply with just the word heron.')}")
        up = until("the follow-up prompt", lambda: hooks(sid, "UserPromptSubmit", mark2), 60)
        until("Stop after the follow-up", lambda: hooks(sid, "Stop", up[0][0]), 90)
        stops_between = [s for s, _ in hooks(sid, "Stop", up_long[0][0]) if s < up[0][0]]
        daemon = q("SELECT kind FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_DAEMON' "
                   "AND seq > ? AND kind = 'interrupted'", sid, mark2)
        follow_up = json.loads(up[0][1]).get("prompt")
        say(f"  interrupted event: {bool(daemon)}; Stop hooks for the interrupted turn: "
            f"{len(stops_between)}; follow-up submitted as {follow_up!r}; reply: {reply(sid, up[0][0])!r}")

    if "5" in steps:
        say(f"5 ctl kill: {ctl('kill', 'run-node', sid)}")
        start = time.time()
        status = None
        while time.time() - start < 15:
            pump(0.2)
            done, st = os.waitpid(pid, os.WNOHANG)
            if done:
                status = st
                break
        say(f"  wrapper exited: {status is not None} after {time.time() - start:.1f} s")
        ended = until("the session ended", lambda: q("SELECT ended_ms FROM sessions WHERE id = ? AND "
                                                     "ended_ms IS NOT NULL", sid), 15)
        say(f"  session ended in the hub: {bool(ended)}")

    pend = q("SELECT tool_name, kind, reason FROM pending WHERE session_id = ? ORDER BY created_ms", sid)
    say("pending requests: " + ("; ".join(f"{t} {k} {r}" for t, k, r in pend) or "none"))
    hk = q("SELECT kind, COUNT(*) FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
           "GROUP BY kind ORDER BY kind", sid)
    say("hook events: " + ", ".join(f"{k} {n}" for k, n in hk))
finally:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
