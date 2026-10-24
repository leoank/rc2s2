"""RUN-20261019-mvz0r3: plays the person at an `rc2s2 run` terminal and the person holding the
phone, through the rc2s2 app on the emulator, then summarises the hub's record of this one
session. Terminal helpers as in RUN-20261019-e78rne's driver; the app is driven through
uiautomator dumps and adb input. Prints no content from any other session."""
import argparse, fcntl, json, os, pty, re, select, signal, sqlite3, struct, subprocess, termios, time

ap = argparse.ArgumentParser()
for a in ["bin", "socket", "settings", "work", "out", "db", "claude", "log", "token-file", "shots"]:
    ap.add_argument("--" + a, required=True)
A = ap.parse_args()
ANSI = re.compile(rb"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)|\x1bP[^\x1b]*\x1b\\|\x1b[()][A-Z0-9]|\x1b[=>]")

# --- the terminal ------------------------------------------------------------------------
env = {k: v for k, v in os.environ.items() if not (k.startswith("CLAUDE") or k == "AI_AGENT")}
env["TERM"] = "xterm-256color"
pid, fd = pty.fork()
if pid == 0:
    os.chdir(A.work)
    os.execve(f"{A.bin}/rc2s2", ["rc2s2", "run", "--command", A.claude, "--wrapper-socket", A.socket,
                                 "--", "--settings", A.settings, "--model", "haiku"], env)
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

def until(what, fn, timeout=120):
    end = time.time() + timeout
    while time.time() < end:
        v = fn()
        if v:
            return v
        pump(0.2)
    raise SystemExit(f"timed out waiting for {what}")

# --- the hub's record ----------------------------------------------------------------------
def q(sql, *a):
    with sqlite3.connect(f"file:{A.db}?mode=ro", uri=True) as c:
        return c.execute(sql, a).fetchall()

def hooks(sid, kind, after=0):
    return q("SELECT seq, payload FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
             "AND kind = ? AND seq > ? ORDER BY seq", sid, kind, after)

def last_seq(sid):
    return q("SELECT COALESCE(MAX(seq), 0) FROM events WHERE session_id = ?", sid)[0][0]

def _reply(sid, after):
    texts = []
    for (payload,) in q("SELECT payload FROM events WHERE session_id = ? AND kind = 'assistant' "
                        "AND source = 'EVENT_SOURCE_TRANSCRIPT' AND seq > ? ORDER BY seq", sid, after):
        for block in json.loads(payload).get("message", {}).get("content", []):
            if isinstance(block, dict) and block.get("type") == "text":
                texts.append(block["text"].strip())
    return texts[-1] if texts else ""

def reply(sid, after):
    # The daemon reads the transcript every 500 ms, so the text lands after the Stop hook.
    return until("the reply in the transcript", lambda: _reply(sid, after), 15)

def open_pending(sid, tool):
    return q("SELECT id, tool_input FROM pending WHERE session_id = ? AND tool_name = ? "
             "AND closed_ms IS NULL", sid, tool)

def closed(p_id):
    return q("SELECT reason, resolution FROM pending WHERE id = ? AND closed_ms IS NOT NULL", p_id)

# --- the app ---------------------------------------------------------------------------------
def adb(*args):
    return subprocess.run(["adb", "-s", "emulator-5584", *args], capture_output=True, text=True).stdout

def views():
    adb("shell", "uiautomator", "dump", "/sdcard/ui.xml")
    xml = adb("shell", "cat", "/sdcard/ui.xml")
    out = []
    for n in re.findall(r"<node [^>]*>", xml):
        g = lambda k: (re.search(rf' {k}="([^"]*)"', n) or [None, ""])[1]
        x1, y1, x2, y2 = map(int, re.findall(r"\d+", g("bounds")))
        out.append((g("text"), g("content-desc"), ((x1 + x2) // 2, (y1 + y2) // 2)))
    return out

def find(text, exact=True):
    for t, d, xy in views():
        if (text in (t, d)) if exact else (text in t or text in d):
            return xy
    return None

def tap(text, exact=True, timeout=30, scroll=False):
    end = time.time() + timeout
    while time.time() < end:
        xy = find(text, exact)
        if xy:
            adb("shell", "input", "tap", str(xy[0]), str(xy[1]))
            return
        if scroll:
            adb("shell", "input", "swipe", "540", "1800", "540", "900", "300")
        sleep(1)
    raise SystemExit(f"no {text!r} in the app")

def type_text(text):
    # adb input text needs %s for a space; the texts here avoid shell metacharacters.
    adb("shell", "input", "text", text.replace(" ", "%s"))

def hide_keyboard():
    if "mInputShown=true" in adb("shell", "dumpsys", "input_method"):
        adb("shell", "input", "keyevent", "KEYCODE_BACK")
        sleep(1)

def shot(name):
    with open(os.path.join(A.shots, name + ".png"), "wb") as f:
        f.write(subprocess.run(["adb", "-s", "emulator-5584", "exec-out", "screencap", "-p"],
                               capture_output=True).stdout)

def app_shows(text):
    return any(text in t for t, _, _ in views())

out = []
def say(line):
    out.append(line); print(line, flush=True)

try:
    until("the prompt box", lambda: b"/effort" in screen or b"shortcuts" in screen, 90)
    sleep(2)
    sid = until("the session wrapped", lambda: q(
        "SELECT id FROM sessions WHERE launch = 'LAUNCH_WRAPPER'"), 30)[0][0]
    work_name = os.path.basename(A.work)
    say(f"session ready under the wrapper after {time.time() - t0:.1f} s")

    # The app connects with the client token.
    adb("shell", "am", "start", "-n", "me.leoank.rc2s2/.MainActivity")
    tap("Client token")
    type_text(open(A.token_file).read().strip())
    hide_keyboard()
    tap("Connect")
    until("the app's session list", lambda: app_shows("Sessions"), 30)
    tap(work_name, scroll=True)  # opened and closed again, to check it is listed
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    say("app connected; the wrapped session is listed")

    # 1. A question, answered by tapping its second option in the app.
    mark = last_seq(sid)
    typed("Use the AskUserQuestion tool to ask me one question: which colour do I prefer, "
          "with the options red and blue. Then reply with just the colour I chose.")
    p_id, tool_input = until("the question held", lambda: open_pending(sid, "AskUserQuestion"))[0]
    option = json.loads(tool_input)["questions"][0]["options"][1]["label"]
    tap(option)
    shot("1-question")
    tap("Answer")
    until("Stop after the answer", lambda: hooks(sid, "Stop", mark))
    say(f"1 question: tapped {option!r}; closed {closed(p_id)[0][0]}; reply {reply(sid, mark)!r}")

    # 2. A command outside the working directory, denied from the app with a message.
    mark = last_seq(sid)
    target = os.path.join(A.out, "denied.txt")
    typed(f"Run exactly this one shell command and nothing else: touch {target}")
    p_id, _ = until("the command held", lambda: open_pending(sid, "Bash"))[0]
    tap("Message if denied")
    type_text("Denied from the phone: use the work directory instead.")
    hide_keyboard()
    shot("2-deny")
    tap("Deny")
    until("Stop after the denial", lambda: hooks(sid, "Stop", mark))
    r = reply(sid, mark)
    say(f"2 deny: closed {closed(p_id)[0][0]}; file exists {os.path.exists(target)}; "
        f"reply quotes the message {'work directory' in r}; reply {r!r}")

    # 3. A prompt sent from the session screen.
    tap(work_name, scroll=True)
    until("the conversation", lambda: app_shows("Claude"), 20)
    mark = last_seq(sid)
    tap("Prompt")
    type_text("Reply with just the word pelican.")
    hide_keyboard()
    tap("Send")
    until("Stop after the app's prompt", lambda: hooks(sid, "Stop", mark), 120)
    r = reply(sid, mark)
    sleep(2)
    shot("3-prompt")
    say(f"3 prompt from the app: UserPromptSubmit {bool(hooks(sid, 'UserPromptSubmit', mark))}; "
        f"reply {r!r}; shown in the app {app_shows('pelican')}")

    # 4. A long turn interrupted from the app with a follow-up.
    mark = last_seq(sid)
    typed("Count from 1 to 3000, one number per line, nothing else, no tools.")
    up_long = until("the long turn to start", lambda: hooks(sid, "UserPromptSubmit", mark))
    sleep(3)
    running = not hooks(sid, "Stop", up_long[0][0])
    mark2 = last_seq(sid)
    tap("Prompt")
    type_text("Reply with just the word heron.")
    hide_keyboard()
    tap("Interrupt, then send")
    up = until("the follow-up prompt", lambda: hooks(sid, "UserPromptSubmit", mark2), 60)
    until("Stop after the follow-up", lambda: hooks(sid, "Stop", up[0][0]), 90)
    stops_between = [s for s, _ in hooks(sid, "Stop", up_long[0][0]) if s < up[0][0]]
    follow_up = json.loads(up[0][1]).get("prompt")
    say(f"4 interrupt from the app: turn running {running}; Stop hooks for it {len(stops_between)}; "
        f"follow-up {follow_up!r}; reply {reply(sid, up[0][0])!r}")
    shot("4-interrupt")

    # 5. Kill from the app.
    tap("Kill")
    start = time.time()
    while time.time() - start < 15:
        pump(0.2)
        if os.waitpid(pid, os.WNOHANG)[0]:
            break
    say(f"5 kill from the app: wrapper exited after {time.time() - start:.1f} s")

    pend = q("SELECT tool_name, kind, reason, resolution FROM pending WHERE session_id = ? "
             "ORDER BY created_ms", sid)
    say("pending requests: " + "; ".join(f"{t} {k} {r} {res}" for t, k, r, res in pend))
    hk = q("SELECT kind, COUNT(*) FROM events WHERE session_id = ? AND source = 'EVENT_SOURCE_HOOK' "
           "GROUP BY kind ORDER BY kind", sid)
    say("hook events: " + ", ".join(f"{k} {n}" for k, n in hk))
except SystemExit as e:
    shot("failed")
    say(f"{e}")
    raise
finally:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
