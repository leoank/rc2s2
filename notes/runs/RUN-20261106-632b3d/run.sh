#!/usr/bin/env bash
# EXP-20261106-62st4b: resume a finished session headless, as the spawner would.
# Run from the repo root: notes/runs/RUN-20261106-632b3d/run.sh [claude binary]
# Under target/live-work-resume-exp, which the owner's live daemon ignores.
set -euo pipefail
exec python3 -I - "$PWD" "${1:-claude}" <<'PY'
import json, os, glob, subprocess, sys, time, uuid, pathlib, threading
root, claude = sys.argv[1], sys.argv[2]
out = pathlib.Path(root, "notes/runs/RUN-20261106-632b3d")
work = pathlib.Path(root, "target/live-work-resume-exp"); work.mkdir(parents=True, exist_ok=True)
enc = str(work).replace("/", "-").replace(".", "-").replace("_", "-")
proj = pathlib.Path(os.path.expanduser(f"~/.claude/projects/{enc}"))
a = str(uuid.uuid4())
res = {"claude": subprocess.run([claude, "--version"], capture_output=True, text=True).stdout.strip(), "original": a}

# 1. The original session: one turn, then it ends.
first = subprocess.run([claude, "-p", "--session-id", a, "--model", "haiku", "--output-format", "json",
    "Remember the code word kiwi. Reply with just: noted."], cwd=work, capture_output=True, text=True)
res["first_reply"] = json.loads(first.stdout).get("result")
before = {p.name: sum(1 for _ in open(p)) for p in proj.glob("*.jsonl")}
res["files_before"] = before

# 2. Resumed headless over stream-json, as the spawner would.
p = subprocess.Popen([claude, "-p", "--input-format", "stream-json", "--output-format", "stream-json",
    "--verbose", "--resume", a, "--model", "haiku"], cwd=work, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
lines = []
def read():
    for l in p.stdout: lines.append(json.loads(l))
threading.Thread(target=read, daemon=True).start()
time.sleep(4)
reg = None
for f in glob.glob(os.path.expanduser("~/.claude/sessions/*.json")):
    try: r = json.load(open(f))
    except Exception: continue
    if r.get("pid") == p.pid: reg = {k: r.get(k) for k in ("sessionId", "status", "entrypoint")}
res["registry_before_prompt"] = reg
p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": "What was the code word? One word."}}) + "\n"); p.stdin.flush()
for _ in range(120):
    if any(m.get("type") == "result" for m in lines): break
    time.sleep(0.5)
res["init_session_ids"] = sorted({m.get("session_id") for m in lines if m.get("type") == "system" and m.get("subtype") == "init"})
res["resumed_reply"] = next((m.get("result") for m in lines if m.get("type") == "result"), None)
for f in glob.glob(os.path.expanduser("~/.claude/sessions/*.json")):
    try: r = json.load(open(f))
    except Exception: continue
    if r.get("pid") == p.pid: reg = {k: r.get(k) for k in ("sessionId", "status", "entrypoint")}
res["registry_after_prompt"] = reg
p.stdin.close(); res["exit"] = p.wait(timeout=30)
after = {p_.name: sum(1 for _ in open(p_)) for p_ in proj.glob("*.jsonl")}
res["files_after"] = after
res["same_id"] = res["init_session_ids"] == [a]
res["remembers"] = "kiwi" in (res["resumed_reply"] or "").lower()
res["appended_to_original"] = after.get(f"{a}.jsonl", 0) > before.get(f"{a}.jsonl", 0)
json.dump(lines, open(out / "stream.json", "w"))
print(json.dumps(res, indent=1))
print("===RESULT===", json.dumps({k: res[k] for k in ("same_id", "remembers", "appended_to_original")}))
PY
