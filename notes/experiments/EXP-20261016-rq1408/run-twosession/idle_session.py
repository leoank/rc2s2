# a long-lived headless session: turn 1, then wait for a file named `go` next to this script's
# output dir, then turn 2. prints each result line's type/subtype/is_error/result and the time.
import json, os, subprocess, sys, time
cfg, outdir = sys.argv[1], sys.argv[2]
env = {k: v for k, v in os.environ.items() if not k.startswith("CLAUDE") and k != "AI_AGENT"}
env["CLAUDE_CONFIG_DIR"] = cfg
p = subprocess.Popen(["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--model", "sonnet"],
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(os.path.join(outdir, "idle.stderr"), "w"), text=True, env=env, cwd=outdir)
log = open(os.path.join(outdir, "idle.jsonl"), "w")

def turn(n):
    p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": f"Reply with just the word ok{n}."}}) + "\n"); p.stdin.flush()
    for line in p.stdout:
        log.write(line); log.flush()
        m = json.loads(line)
        if m.get("type") == "result":
            print(time.strftime("%H:%M:%S"), f"turn {n}:", {k: m.get(k) for k in ("subtype", "is_error", "result", "duration_ms")}, flush=True)
            return
    print(time.strftime("%H:%M:%S"), f"turn {n}: stream ended, exit", p.poll(), flush=True)

turn(1)
go = os.path.join(outdir, "go")
print(time.strftime("%H:%M:%S"), "waiting for", go, flush=True)
while not os.path.exists(go):
    time.sleep(0.5)
turn(2)
p.stdin.close(); p.wait()
print("exit", p.returncode)
