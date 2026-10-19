#!/usr/bin/env python3
"""Throwaway hook sink for EXP-0001/0003/0004.

Receives Claude Code HTTP hooks on 127.0.0.1:PORT, appends every event to
RUNDIR/events.jsonl, and:
  - PermissionRequest: waits up to HOLD seconds for RUNDIR/answer.json, returns its
    content (and removes it), else returns {}.
  - PreToolUse for AskUserQuestion: if RUNDIR/ask_answer.json exists, returns it.
  - everything else: returns {} immediately.
Usage: hook_server.py RUNDIR [PORT] [HOLD]
"""
import json, os, sys, time
from http.server import BaseHTTPRequestHandler, HTTPServer

RUNDIR = sys.argv[1]
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 7777
HOLD = float(sys.argv[3]) if len(sys.argv) > 3 else 60
os.makedirs(RUNDIR, exist_ok=True)
LOG = os.path.join(RUNDIR, "events.jsonl")


def log(kind, obj):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), "kind": kind, **obj}) + "\n")


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(n)
        try:
            ev = json.loads(raw)
        except Exception:
            ev = {"raw": raw.decode("utf8", "replace")}
        log("in", {"path": self.path, "headers": dict(self.headers), "event": ev})
        name = ev.get("hook_event_name")
        resp = {}
        mode_file = os.path.join(RUNDIR, "ask_mode")
        ask_mode = open(mode_file).read().strip() if os.path.exists(mode_file) else ""
        if ev.get("tool_name") == "AskUserQuestion" and (
            (name == "PreToolUse" and ask_mode == "pre") or (name == "PermissionRequest" and ask_mode == "perm")):
            ti = ev.get("tool_input", {})
            answers = {q["question"]: q["options"][1]["label"] for q in ti.get("questions", [])}
            upd = dict(ti, answers=answers)
            if name == "PreToolUse":
                resp = {"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": upd}}
            else:
                resp = {"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow", "updatedInput": upd}}}
        elif name == "PermissionRequest":
            ans = os.path.join(RUNDIR, "answer.json")
            t0 = time.time()
            import select, socket
            gone = False
            while time.time() - t0 < HOLD:
                if os.path.exists(ans):
                    resp = json.load(open(ans))
                    os.remove(ans)
                    break
                r, _, _ = select.select([self.connection], [], [], 0.2)
                if r:
                    try:
                        if self.connection.recv(1, socket.MSG_PEEK) == b"":
                            gone = True
                    except OSError:
                        gone = True
                    if gone:
                        log("client_closed_during_hold", {"after": time.time() - t0})
                        break
            log("hold", {"waited": time.time() - t0, "answered": bool(resp), "client_gone": gone})
        elif name == "PreToolUse" and ev.get("tool_name") == "AskUserQuestion":
            ans = os.path.join(RUNDIR, "ask_answer.json")
            if os.path.exists(ans):
                resp = json.load(open(ans))
        body = json.dumps(resp).encode()
        log("out", {"event": name, "response": resp})
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        try:
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError) as e:
            log("client_gone", {"event": name, "error": repr(e)})


print(f"hook server on 127.0.0.1:{PORT}, log {LOG}, hold {HOLD}s", flush=True)
HTTPServer(("127.0.0.1", PORT), H).serve_forever()
