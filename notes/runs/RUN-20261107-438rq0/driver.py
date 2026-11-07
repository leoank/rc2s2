#!/usr/bin/env python3
"""RUN-20261107-438rq0: a headless stream-json claude session under mxc seatbelt, driven the
way the node's spawner drives one (EXP-20261107-413q82).

Usage: driver.py <mxc-stream binary> <token file> <out dir>

Builds a sandbox under /private/tmp/rc2s2-sbx (short, for the 104-byte sun_path limit), a
launch policy from the profile below, a settings file of command hooks posting over a unix
socket, and runs three sessions through mxc-stream:
  A. turn 1 probes the inside (nix develop, git identity, SSH agent, tools, and reads that
     must be refused) with every PermissionRequest held 1 s and allowed over the socket;
     turn 2 starts a long Bash and is interrupted with a control request; stdin closes.
  B. `--resume <A's id>` asks for the code word turn 1 was given.
  C. `--resume` again, a long Bash, then a kill through MxcProcess::kill.
Writes stream.A/B/C.jsonl, hooks.jsonl, a listing of the sandbox config dir, and prints one
`===RESULT=== {...}` line. The token is read from the file and passed in the policy's
environment only; the policy file is mode 600 and deleted once the bridge has read it.
"""
import json, os, pathlib, shutil, socketserver, subprocess, sys, threading, time
from http.server import BaseHTTPRequestHandler

BRIDGE, TOKEN_FILE, OUT = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
OUT.mkdir(parents=True, exist_ok=True)
HOME = pathlib.Path("/Users/ank")
REPO = HOME / "workspace/hub/leoank/rc2s2"
BASE = pathlib.Path("/private/tmp/rc2s2-sbx")
CONFIG, TMP, SHOME, HOOKS, WORK = (BASE / d for d in ("config", "tmp", "home", "hooks", "work"))
SOCK = HOOKS / "hook.sock"
CODE = "SAFFRON-41"
EVENTS = ["SessionStart", "UserPromptSubmit", "PreToolUse", "PermissionRequest", "PostToolUse",
          "Notification", "Stop", "SubagentStop", "SessionEnd"]

# --- the sandbox's directories, user Claude files and the hook settings -------------------
shutil.rmtree(BASE, ignore_errors=True)
for d in (CONFIG, TMP, SHOME, HOOKS, WORK):
    d.mkdir(parents=True)
subprocess.run(["git", "clone", "-q", str(REPO), str(WORK / "rc2s2")], check=True)
USER_CLAUDE = ["CLAUDE.md", "skills", "commands", "agents"]
for name in USER_CLAUDE:
    if (HOME / ".claude" / name).exists():
        (CONFIG / name).symlink_to(HOME / ".claude" / name)
curl = (f"/usr/bin/curl -s --max-time 600 --unix-socket {SOCK} -X POST "
        "-H 'Content-Type: application/json' --data-binary @- http://hook/hook")
hooks = {e: [{"hooks": [{"type": "command", "command": curl, "timeout": 600}]}] for e in EVENTS}
SETTINGS = CONFIG / "rc2s2-hooks.json"
SETTINGS.write_text(json.dumps({"hooks": hooks}))

PATH = ":".join([str(HOME / ".nix-profile/bin"), "/etc/profiles/per-user/ank/bin",
                 "/run/current-system/sw/bin", "/nix/var/nix/profiles/default/bin",
                 "/usr/bin", "/bin", "/usr/sbin", "/sbin"])
POLICY = {
    "readonly": ["/nix/store", "/nix/var/nix/profiles", "/nix/var/nix/gcroots",
                 "/private/etc/profiles", "/private/etc/nix", "/private/etc/static",
                 "/private/var/run/current-system",
                 str(HOME / ".nix-profile"), str(HOME / ".local/state/nix/profiles"),
                 str(HOME / ".config/git"), str(HOME / ".ssh/config"), str(HOME / ".ssh/known_hosts")]
                + [str(HOME / ".claude" / n) for n in USER_CLAUDE],
    "readwrite": [str(d) for d in (CONFIG, TMP, SHOME, HOOKS, WORK)],
    "denied": ["/private/tmp/cc-socks"],
    "egress": "allow", "ingress": "deny", "hostLoopback": "deny",
    "extraMachLookups": ["com.apple.system.opendirectoryd.libinfo"],
    "env": {
        "PATH": PATH, "HOME": str(SHOME), "USER": "ank", "LANG": "en_US.UTF-8", "TERM": "dumb",
        "CLAUDE_CONFIG_DIR": str(CONFIG), "CLAUDE_CODE_TMPDIR": str(TMP), "TMPDIR": str(TMP),
        "XDG_CACHE_HOME": str(SHOME / ".cache"),
        "GIT_CONFIG_GLOBAL": str(HOME / ".config/git/config"),
        "SSH_AUTH_SOCK": str(HOME / ".bitwarden-ssh-agent.sock"),
        "CLAUDE_CODE_OAUTH_TOKEN": pathlib.Path(TOKEN_FILE).read_text().strip() if TOKEN_FILE != "-" else "",
    },
    "cwd": str(WORK / "rc2s2"),
}
if TOKEN_FILE == "-":  # the policy alone, for probing the sandbox with a shell: driver.py - - <out>
    print(json.dumps(POLICY))
    sys.exit(0)
public = {k: v for k, v in POLICY.items() if k != "env"}
public["env"] = {k: ("<redacted>" if "TOKEN" in k else v) for k, v in POLICY["env"].items()}
(OUT / "policy.json").write_text(json.dumps(public, indent=1))

# --- the hook socket: log every event, hold each PermissionRequest 1 s and allow ---------
hook_log = open(OUT / "hooks.jsonl", "a")
ALLOW = {"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}}
perms = []


class Hook(BaseHTTPRequestHandler):
    def address_string(self):
        return "unix"

    def log_message(self, *a):
        pass

    def do_POST(self):
        ev = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
        name = ev.get("hook_event_name")
        resp = {}
        if name == "PermissionRequest":
            time.sleep(1)
            resp = ALLOW
            perms.append((time.time(), ev.get("tool_name"), ev.get("tool_input")))
        hook_log.write(json.dumps({"t": time.time(), "event": name, "session": ev.get("session_id"),
                                   "tool": ev.get("tool_name"), "resp": resp}) + "\n")
        hook_log.flush()
        body = json.dumps(resp).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


server = Server(str(SOCK), Hook)
threading.Thread(target=server.serve_forever, daemon=True).start()


# --- a session through the bridge -----------------------------------------------------------
class Session:
    def __init__(self, tag, extra):
        pol = BASE / f"policy-{tag}.json"
        pol.touch(mode=0o600)
        pol.write_text(json.dumps(POLICY))
        cmd = [BRIDGE, str(pol), "--", "claude", "-p", "--input-format", "stream-json",
               "--output-format", "stream-json", "--verbose", "--model", "haiku",
               "--settings", str(SETTINGS)] + extra
        self.err = open(OUT / f"bridge.{tag}.log", "w")
        self.p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.err, text=True)
        time.sleep(2)
        pol.unlink()
        self.log = open(OUT / f"stream.{tag}.jsonl", "w")
        self.lines, self.sid, self.t0 = [], None, time.time()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        for line in self.p.stdout:
            self.log.write(line)
            self.log.flush()
            try:
                m = json.loads(line)
            except ValueError:
                continue
            self.sid = self.sid or m.get("session_id")
            self.lines.append((time.time(), m))

    def send(self, obj):
        self.p.stdin.write((obj if isinstance(obj, str) else json.dumps(obj)) + "\n")
        self.p.stdin.flush()

    def prompt(self, text):
        self.send({"type": "user", "message": {"role": "user", "content": text}})

    def wait(self, pred, timeout):
        end = time.time() + timeout
        seen = 0
        while time.time() < end:
            for _, m in self.lines[seen:]:
                if pred(m):
                    return m
            seen = 0
            time.sleep(0.2)
        return None

    def results(self):
        return [m for _, m in self.lines if m.get("type") == "result"]

    def text(self):
        out = []
        for _, m in self.lines:
            if m.get("type") == "assistant":
                out += [c.get("text", "") for c in m["message"]["content"] if c.get("type") == "text"]
        return "\n".join(out)

    def close(self, timeout=60):
        self.p.stdin.close()
        try:
            return self.p.wait(timeout)
        except subprocess.TimeoutExpired:
            return None


def nth_result(s, n, timeout):
    return s.wait(lambda m: m.get("type") == "result" and len(s.results()) >= n, timeout)


result = {}
PROBE = f"""This is a sandbox test; the commands are expected to fail in places. Remember the
code word {CODE} for later. Run each of these shell commands with the Bash tool, one call per
command, and then reply with a table of command and its exact output or error:
1. nix develop -c rustc --version
2. git config user.name
3. ssh-add -l | cut -c1-24
4. ssh -T -o BatchMode=yes -o ConnectTimeout=15 git@github.com
5. command -v rg gh yajna jq
6. wc -c < /Users/ank/.ssh/id_ed25519
7. ls /Users/ank/Documents | head -3
8. ls /private/tmp/cc-socks | head -3
9. wc -c < /Users/ank/.claude/.credentials.json
10. curl -s -m 5 -o /dev/null -w '%{{http_code}}' http://127.0.0.1:7797/ ; echo
11. curl -s -m 10 -o /dev/null -w '%{{http_code}}' https://api.github.com ; echo
"""

# A: probes, then an interrupt mid-Bash, then end of input.
A = Session("A", [])
A.prompt(PROBE)
r1 = nth_result(A, 1, 600)
result["A_turn1"] = bool(r1) and r1.get("subtype")
result["A_session_id"] = A.sid
time.sleep(1)
result["registry_while_alive"] = sorted(p.name for p in (CONFIG / "sessions").glob("*.json")) if (CONFIG / "sessions").exists() else []
n_perm = len(perms)
A.prompt("Run exactly this with the Bash tool, in the foreground, and nothing else: python3 -c 'import time; time.sleep(45); print(\"finished-45\")'")
t_wait = time.time()
while len(perms) == n_perm and time.time() - t_wait < 120:
    time.sleep(0.2)
time.sleep(4)
result["A_tool_running_before_interrupt"] = bool(subprocess.run(["pgrep", "-f", "time.sleep\\(45\\)"], capture_output=True).stdout)
t_int = time.time()
A.send({"type": "control_request", "request_id": "int-1", "request": {"subtype": "interrupt"}})
r2 = nth_result(A, 2, 60)
result["A_tool_left_after_interrupt"] = bool(subprocess.run(["pgrep", "-f", "time.sleep\\(45\\)"], capture_output=True).stdout)
result["A_interrupt_result_after_s"] = r2 and round(time.time() - t_int, 1)
result["A_interrupt_ack"] = bool(A.wait(lambda m: m.get("type") == "control_response", 5))
result["A_exit_on_close"] = A.close()
(OUT / "answer.A.md").write_text(A.text())

# B: resume, the code word.
B = Session("B", ["--resume", A.sid])
B.prompt("What was the code word I gave you earlier? Answer with the word only.")
nth_result(B, 1, 180)
result["B_session_id"] = B.sid
result["B_code_word_kept"] = CODE in B.text()
result["B_exit_on_close"] = B.close()

# C: resume, a long Bash, killed through mxc.
C = Session("C", ["--resume", A.sid])
n_perm = len(perms)
C.prompt("Run exactly this with the Bash tool, in the foreground, and nothing else: python3 -c 'import time; time.sleep(120); print(\"finished-120\")'")
t_wait = time.time()
while len(perms) == n_perm and time.time() - t_wait < 120:
    time.sleep(0.2)
time.sleep(3)
result["C_tool_running_before_kill"] = bool(subprocess.run(["pgrep", "-f", "time.sleep\\(120\\)"], capture_output=True).stdout)
t_kill = time.time()
C.send("__KILL__")
try:
    result["C_exit_after_kill"] = C.p.wait(30)
except subprocess.TimeoutExpired:
    result["C_exit_after_kill"] = None
result["C_kill_s"] = round(time.time() - t_kill, 1)
leftover = subprocess.run(["pgrep", "-f", "time.sleep\\(120\\)"], capture_output=True, text=True).stdout.split()
result["C_leftover_pids"] = leftover

# The config dir afterwards.
listing = sorted(str(p.relative_to(CONFIG)) for p in CONFIG.rglob("*") if p.is_file() and ".claude.json" not in p.name)
(OUT / "config-listing.txt").write_text("\n".join(listing) + "\n")
result["transcript_in_config"] = any(A.sid in p and p.endswith(".jsonl") for p in listing)
result["credential_file_in_config"] = any(p.endswith(".credentials.json") for p in listing)
result["permission_requests_allowed"] = len(perms)
result["hook_events"] = sorted({json.loads(l)["event"] for l in open(OUT / "hooks.jsonl")})
print("===RESULT=== " + json.dumps(result))
server.shutdown()
