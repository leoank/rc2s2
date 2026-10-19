#!/usr/bin/env python3
"""Hook sink variant for EXP-0004 that answers AskUserQuestion from a file.

Like hook_server.py, but the AskUserQuestion answer map is read verbatim from
RUNDIR/ask_answers.json (a JSON object: {<question text>: <value>}), so the
value can be any shape the tool accepts (string, or array of labels for a
multiSelect question, or arbitrary free text for an "Other" answer). The map
is dropped straight into updatedInput.answers.

  - AskUserQuestion, ask_mode "pre"  -> answered on PreToolUse.
  - AskUserQuestion, ask_mode "perm" -> answered on PermissionRequest.
  - every other event: {} immediately.
Each event (in and out) is appended to RUNDIR/events.jsonl.
Usage: hook_server_ask.py RUNDIR [PORT]
"""
import json, os, sys, time
from http.server import BaseHTTPRequestHandler, HTTPServer

RUNDIR = sys.argv[1]
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 7792
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
        log("in", {"path": self.path, "event": ev})
        name = ev.get("hook_event_name")
        resp = {}
        mode_file = os.path.join(RUNDIR, "ask_mode")
        ask_mode = open(mode_file).read().strip() if os.path.exists(mode_file) else ""
        ans_file = os.path.join(RUNDIR, "ask_answers.json")
        if ev.get("tool_name") == "AskUserQuestion" and os.path.exists(ans_file) and (
            (name == "PreToolUse" and ask_mode == "pre")
            or (name == "PermissionRequest" and ask_mode == "perm")
        ):
            spec = json.load(open(ans_file))
            ti = ev.get("tool_input", {})
            qs = ti.get("questions", [])
            # dict: {question text: value} used verbatim. list: positional,
            # keyed by each question's actual text (robust to rephrasing).
            if isinstance(spec, list):
                answers = {qs[i]["question"]: spec[i] for i in range(min(len(qs), len(spec)))}
            else:
                answers = spec
            upd = dict(ti, answers=answers)
            if name == "PreToolUse":
                resp = {"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                               "permissionDecision": "allow",
                                               "updatedInput": upd}}
            else:
                resp = {"hookSpecificOutput": {"hookEventName": "PermissionRequest",
                                               "decision": {"behavior": "allow",
                                                            "updatedInput": upd}}}
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


print(f"hook server (ask) on 127.0.0.1:{PORT}, log {LOG}", flush=True)
HTTPServer(("127.0.0.1", PORT), H).serve_forever()
