"""RUN-20261025-166zqd: plays the person at an `rc2s2 run` terminal and the person holding the
phone, through the rebuilt rc2s2 app on the emulator, then summarises the hub's record of this
one session. The terminal and hub helpers are RUN-20261019-mvz0r3's; the app side follows the
new screens: the deny dialog, the stop button, the kill dialog. Prints no content from any
other session."""
import argparse, fcntl, json, os, pty, re, select, signal, sqlite3, struct, subprocess, termios, time

ap = argparse.ArgumentParser()
for a in ["bin", "socket", "settings", "work", "out", "db", "claude", "log", "token-file", "shots", "device"]:
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
    return subprocess.run(["adb", "-s", A.device, *args], capture_output=True, text=True).stdout

def views():
    adb("shell", "uiautomator", "dump", "/sdcard/ui.xml")
    xml = adb("shell", "cat", "/sdcard/ui.xml")
    out = []
    for n in re.findall(r"<node [^>]*>", xml):
        g = lambda k: (re.search(rf' {k}="([^"]*)"', n) or [None, ""])[1]
        nums = list(map(int, re.findall(r"\d+", g("bounds"))))
        if len(nums) < 4:
            continue
        x1, y1, x2, y2 = nums[:4]
        out.append((g("text"), g("content-desc"), ((x1 + x2) // 2, (y1 + y2) // 2)))
    return out

def find(text, exact=True, last=False):
    hits = [xy for t, d, xy in views() if ((text in (t, d)) if exact else (text in t or text in d))]
    return (hits[-1] if last else hits[0]) if hits else None

def tap(text, exact=True, timeout=30, scroll=False, last=False):
    """Taps the view with this text or description; `last` picks a dialog's button over a
    card's of the same name, which stays in the hierarchy under the scrim."""
    end = time.time() + timeout
    while time.time() < end:
        xy = find(text, exact, last)
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

def replace_field(label, text):
    """Selects all in the focused field (the one just tapped by its label) and types over it."""
    tap(label)
    adb("shell", "input", "keycombination", "KEYCODE_CTRL_LEFT", "KEYCODE_A")
    adb("shell", "input", "keyevent", "KEYCODE_DEL")
    type_text(text)

def hide_keyboard():
    if "mInputShown=true" in adb("shell", "dumpsys", "input_method"):
        adb("shell", "input", "keyevent", "KEYCODE_BACK")
        sleep(1)

def shot(name):
    with open(os.path.join(A.shots, name + ".png"), "wb") as f:
        f.write(subprocess.run(["adb", "-s", A.device, "exec-out", "screencap", "-p"],
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

    # The app connects with the client token; the address field holds the real hub's by default.
    adb("shell", "am", "start", "-n", "me.leoank.rc2s2/.MainActivity")
    sleep(2)
    replace_field("Hub address", "http://10.0.2.2:7700")
    tap("Client token")
    type_text(open(A.token_file).read().strip())
    hide_keyboard()
    tap("Connect")
    until("the app's session list", lambda: app_shows("Sessions"), 30)
    shot("0-home")
    tap(work_name, scroll=True)
    until("the session screen", lambda: app_shows("Prompt"), 20)
    say("app connected; the wrapped session is listed and opens")

    # 1. A question, answered by tapping its second option on the session screen.
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

    # 2. A command outside the working directory, denied from the home screen with a reason.
    mark = last_seq(sid)
    target = os.path.join(A.out, "denied.txt")
    typed(f"Run exactly this one shell command and nothing else: touch {target}")
    p_id, _ = until("the command held", lambda: open_pending(sid, "Bash"))[0]
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    until("the request on the home screen", lambda: app_shows("Waiting for you"), 20)
    shot("2-home-waiting")
    tap("Deny")
    tap("Reason (optional)")
    type_text("Denied from the phone: use the work directory instead.")
    hide_keyboard()
    shot("2-deny")
    tap("Deny", last=True)
    until("Stop after the denial", lambda: hooks(sid, "Stop", mark))
    r = reply(sid, mark)
    say(f"2 deny: closed {closed(p_id)[0][0]}; file exists {os.path.exists(target)}; "
        f"reply quotes the message {'work directory' in r}; reply {r!r}")

    # 3. A prompt sent from the session screen, shown there as what was typed.
    tap(work_name, scroll=True)
    until("the session screen", lambda: app_shows("Prompt"), 20)
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
        f"reply {r!r}; shown in the app {app_shows('pelican')}; "
        f"banner hidden {not app_shows('Another Claude session')}")

    # 4. A long turn interrupted from the app with a follow-up: the stop button with text typed.
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
    sleep(2)
    shot("4-stopped")
    up = until("the follow-up prompt", lambda: hooks(sid, "UserPromptSubmit", mark2), 60)
    until("Stop after the follow-up", lambda: hooks(sid, "Stop", up[0][0]), 90)
    stops_between = [s for s, _ in hooks(sid, "Stop", up_long[0][0]) if s < up[0][0]]
    follow_up = json.loads(up[0][1]).get("prompt")
    sleep(2)
    say(f"4 interrupt from the app: turn running {running}; Stop hooks for it {len(stops_between)}; "
        f"follow-up {follow_up!r}; reply {reply(sid, up[0][0])!r}; "
        f"INTERRUPTED shown {app_shows('INTERRUPTED')}")
    shot("4-interrupt")

    # 5. Kill from the session menu, confirmed in its dialog.
    tap("More")
    tap("Kill session")
    shot("5-kill")
    tap("Kill", last=True)
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
    say(f"{e}; app process {adb('shell', 'pidof', 'me.leoank.rc2s2').strip() or 'gone'}; "
        f"exit info: " + " | ".join(l.strip() for l in adb("shell", "dumpsys", "activity", "exit-info",
                                                         "me.leoank.rc2s2").splitlines() if "reason=" in l)[:300])
    raise
finally:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
