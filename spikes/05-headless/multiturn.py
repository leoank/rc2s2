#!/usr/bin/env python3
"""Drive one `claude -p` process through several user turns over stream-json stdin.
Usage: multiturn.py OUTDIR  (writes stream.jsonl and timeline.txt)"""
import json, os, subprocess, sys, threading, time
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
env = {k: v for k, v in os.environ.items() if not k.startswith("CLAUDE") and k != "AI_AGENT"}
p = subprocess.Popen(["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json",
                      "--verbose", "--permission-mode", "default"],
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(f"{out}/stderr.txt", "w"), env=env, text=True)
events = []; tl = open(f"{out}/timeline.txt", "w"); t0 = time.time()
def note(s): tl.write(f"{time.time()-t0:6.1f} {s}\n"); tl.flush(); print(f"{time.time()-t0:6.1f} {s}", flush=True)
results = []
def reader():
    with open(f"{out}/stream.jsonl", "w") as f:
        for line in p.stdout:
            f.write(line); f.flush()
            try: e = json.loads(line)
            except Exception: continue
            events.append(e)
            t = e.get("type"); s = e.get("subtype", "")
            if t == "result": results.append(e); note(f"result #{len(results)}: {(e.get('result') or '')[:80]!r} session={e.get('session_id')}")
            elif t == "system" and s == "init": note(f"init session={e.get('session_id')} socket={e.get('messaging_socket_path')}")
            elif t == "assistant": note("assistant message")
            elif t == "rate_limit_event": note("rate_limit_event")
            elif t == "system": note(f"system/{s}")
threading.Thread(target=reader, daemon=True).start()
def send(text):
    msg = {"type": "user", "message": {"role": "user", "content": text}}
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush(); note(f"sent: {text[:60]!r}")
send("Remember the word 'pelican'. Reply with just 'noted'.")
while len(results) < 1 and p.poll() is None and time.time() - t0 < 90: time.sleep(0.2)
note("waiting 10 s idle before turn 2")
time.sleep(10)
send("What word did I ask you to remember? Reply with just the word.")
while len(results) < 2 and p.poll() is None and time.time() - t0 < 150: time.sleep(0.2)
note(f"process alive after two results: {p.poll() is None}")
p.stdin.close()
try: p.wait(timeout=30); note(f"exit code {p.returncode} after stdin close")
except subprocess.TimeoutExpired: p.kill(); note("killed: did not exit after stdin close")
