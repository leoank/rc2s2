#!/usr/bin/env bash
# EXP-20261106-37f79k: a headless stream-json session as the daemon would spawn it.
# Run from the repo root: notes/runs/RUN-20261106-37krk4/run.sh [claude binary]
# The session runs under target/live-work-spawn-exp, which the owner's live daemon ignores
# (--ignore-cwd */target/live-work-*), so no hook of it reaches the live hub.
set -euo pipefail
exec python3 -I - "$PWD" "${1:-claude}" <<'PY'
import json, os, glob, subprocess, sys, threading, time, uuid, pathlib

root, claude = sys.argv[1], sys.argv[2]
out = pathlib.Path(root, "notes/runs/RUN-20261106-37krk4")
work = pathlib.Path(root, "target/live-work-spawn-exp")
work.mkdir(parents=True, exist_ok=True)
sid = str(uuid.uuid4())
t0 = time.time()
lines, cond = [], threading.Condition()

p = subprocess.Popen(
    [claude, "-p", "--input-format", "stream-json", "--output-format", "stream-json",
     "--verbose", "--session-id", sid, "--model", "haiku",
     "--allowedTools", "Bash(sleep:*)"],
    cwd=work, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)

def read():
    for line in p.stdout:
        msg = json.loads(line)
        with cond:
            lines.append((round(time.time() - t0, 2), msg))
            cond.notify_all()
threading.Thread(target=read, daemon=True).start()

def send(obj):
    p.stdin.write(json.dumps(obj) + "\n"); p.stdin.flush()

def user(text):
    send({"type": "user", "message": {"role": "user", "content": text}})

def wait_for(pred, timeout):
    end = time.time() + timeout
    with cond:
        while True:
            for t, m in lines:
                if pred(m): return t, m
            left = end - time.time()
            if left <= 0: return None, None
            cond.wait(left)

def results(): return [m for _, m in lines if m.get("type") == "result"]

def registry():
    for f in glob.glob(os.path.expanduser("~/.claude/sessions/*.json")):
        try: r = json.load(open(f))
        except Exception: continue
        if r.get("pid") == p.pid or r.get("sessionId") == sid:
            return {k: r.get(k) for k in ("pid", "sessionId", "cwd", "status", "kind", "entrypoint")} | {"file": os.path.basename(f)}
    return None

res = {"claude": subprocess.run([claude, "--version"], capture_output=True, text=True).stdout.strip(),
       "session_id": sid}
user("Run the shell command `sleep 20` with the Bash tool, then reply with the single word done.")
t_tool, _ = wait_for(lambda m: m.get("type") == "assistant" and any(
    c.get("type") == "tool_use" for c in m["message"]["content"]), 60)
res["tool_use_at_s"] = t_tool
time.sleep(3)
res["registry_while_busy"] = registry()
res["sleep_running_before"] = subprocess.run(["pgrep", "-f", "^sleep 20$"], capture_output=True).returncode == 0
t_int = round(time.time() - t0, 2)
send({"type": "control_request", "request_id": "int-1", "request": {"subtype": "interrupt"}})
_, ack = wait_for(lambda m: m.get("type") == "control_response", 15)
res["interrupt_sent_at_s"] = t_int
res["interrupt_response"] = ack and ack.get("response")
t_r1, r1 = wait_for(lambda m: m.get("type") == "result", 30)
res["turn1_result_at_s"] = t_r1
res["turn1_result"] = r1 and {k: r1.get(k) for k in ("subtype", "is_error", "result", "stop_reason")}
time.sleep(1)
res["sleep_running_after"] = subprocess.run(["pgrep", "-f", "^sleep 20$"], capture_output=True).returncode == 0

user("Reply with the single word pong.")
t_r2, r2 = wait_for(lambda m: m.get("type") == "result" and len(results()) >= 2 and m is results()[1], 60)
res["turn2_result"] = r2 and {k: r2.get(k) for k in ("subtype", "is_error", "result")}
res["registry_while_idle"] = registry()

p.stdin.close()
t_close = time.time()
res["exit_code"] = p.wait(timeout=30)
res["exit_after_close_s"] = round(time.time() - t_close, 2)
time.sleep(1)
res["registry_after_exit"] = registry()
enc = str(work).replace("/", "-").replace(".", "-")
res["transcript_exists"] = pathlib.Path(os.path.expanduser(f"~/.claude/projects/{enc}/{sid}.jsonl")).exists()
res["init_session_id_matches"] = any(m.get("type") == "system" and m.get("subtype") == "init"
                                     and m.get("session_id") == sid for _, m in lines)
res["message_types"] = sorted({f'{m.get("type")}/{m.get("subtype","")}' for _, m in lines})

with open(out / "stream.jsonl", "w") as f:
    for t, m in lines: f.write(json.dumps({"t": t, "msg": m}) + "\n")
print(json.dumps(res, indent=1))
print("===RESULT===", json.dumps({"interrupt_ok": bool(r1) and t_r1 - t_int < 10,
      "in_registry": res["registry_while_busy"] is not None,
      "clean_exit": res["exit_code"] == 0}))
PY
