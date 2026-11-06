#!/usr/bin/env bash
# EXP-20261106-qt94y4: does a headless stream-json `claude -p` ask a PermissionRequest hook
# that reaches it only through --settings, and which flags start it in bypass mode?
# Run from the repo root: notes/runs/RUN-20261106-qtaf57/run.sh [claude binary]
# Works under target/live-work-perm-exp, which the owner's live daemon ignores. Each case
# asks for one Bash command that needs approval and checks whether it ran.
set -euo pipefail
exec python3 -I - "$PWD" "${1:-claude}" <<'PY'
import json, subprocess, sys, threading, time, pathlib, shutil
from http.server import BaseHTTPRequestHandler, HTTPServer
root, claude = sys.argv[1], sys.argv[2]
out = pathlib.Path(root, "notes/runs/RUN-20261106-qtaf57")
base = pathlib.Path(root, "target/live-work-perm-exp")
shutil.rmtree(base, ignore_errors=True)

calls, answer = [], {"behavior": "allow"}
class Hook(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        calls.append({k: body.get(k) for k in ("hook_event_name", "session_id", "tool_name", "permission_mode")})
        reply = json.dumps({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": answer}}).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(reply))); self.end_headers(); self.wfile.write(reply)
    def log_message(self, *a): pass
srv = HTTPServer(("127.0.0.1", 0), Hook)
threading.Thread(target=srv.serve_forever, daemon=True).start()
url = f"http://127.0.0.1:{srv.server_port}/hook"
hooks = {"hooks": {"PermissionRequest": [{"hooks": [{"type": "http", "url": url, "timeout": 60}]}]}}

PROMPT = ("Use the Bash tool to run exactly this command and nothing else: "
          "python3 -c \"open('probe.txt','w').write('ran')\" ; then reply with one word: done.")

def case(name, flags, decision=None, project_hook=False):
    global answer
    work = base / name; work.mkdir(parents=True)
    if project_hook:
        (work / ".claude").mkdir(); (work / ".claude/settings.json").write_text(json.dumps(hooks))
    answer = decision or {"behavior": "allow"}
    calls.clear()
    p = subprocess.Popen([claude, "-p", "--input-format", "stream-json", "--output-format", "stream-json",
        "--verbose", "--model", "haiku", *flags], cwd=work, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True)
    p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": PROMPT}}) + "\n"); p.stdin.flush()
    lines = []
    for l in p.stdout:
        m = json.loads(l); lines.append(m)
        if m.get("type") == "result": break
    p.stdin.close(); p.wait(timeout=60)
    init = next((m for m in lines if m.get("type") == "system" and m.get("subtype") == "init"), {})
    denials = next((m.get("permission_denials") for m in lines if m.get("type") == "result"), None)
    r = {"flags": [f if len(f) < 80 else "<settings json>" for f in flags], "permissionMode": init.get("permissionMode"),
         "hook_calls": list(calls), "ran": (work / "probe.txt").exists(), "denials": len(denials or []),
         "stderr": p.stderr.read().strip()[:300]}
    print(name, json.dumps(r)); return r

res = {"claude": subprocess.run([claude, "--version"], capture_output=True, text=True).stdout.strip()}
s = json.dumps(hooks)
res["no_hook"] = case("no_hook", [])
res["settings_allow"] = case("settings_allow", ["--settings", s])
res["settings_deny"] = case("settings_deny", ["--settings", s], {"behavior": "deny", "message": "no from the phone"})
res["settings_and_project_hook"] = case("settings_and_project_hook", ["--settings", s], project_hook=True)
res["skip_flag"] = case("skip_flag", ["--settings", s, "--dangerously-skip-permissions"])
res["mode_bypass"] = case("mode_bypass", ["--settings", s, "--permission-mode", "bypassPermissions"])
res["mode_accept_edits"] = case("mode_accept_edits", ["--settings", s, "--permission-mode", "acceptEdits"])
res["mode_plan"] = case("mode_plan", ["--settings", s, "--permission-mode", "plan"])
srv.shutdown()
(out / "result.json").write_text(json.dumps(res, indent=1))
print("===RESULT===", json.dumps({k: {"ran": v["ran"], "hook_calls": len(v["hook_calls"]), "mode": v["permissionMode"]}
                                  for k, v in res.items() if k != "claude"}))
PY
