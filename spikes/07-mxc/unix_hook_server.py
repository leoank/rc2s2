#!/usr/bin/env python3
"""Hook sink on an AF_UNIX socket, for EXP-0007 (sandboxed claude, loopback TCP denied).

The sandboxed side posts each hook event with
  /usr/bin/curl -s --unix-socket SOCK -X POST --data-binary @- http://hook/hook
Every event is appended to RUNDIR/events.jsonl. Responses:
  - PermissionRequest: if RUNDIR/answer.json exists its content is returned (and the
    file removed); otherwise the request is held HOLD seconds and then answered with
    an allow decision, so the hold path is measurable without a human.
  - everything else: {} immediately.
Usage: unix_hook_server.py RUNDIR SOCK [HOLD]
"""
import json, os, socketserver, sys, time
from http.server import BaseHTTPRequestHandler

RUNDIR = sys.argv[1]
SOCK = sys.argv[2]
HOLD = float(sys.argv[3]) if len(sys.argv) > 3 else 2.0
os.makedirs(RUNDIR, exist_ok=True)
LOG = os.path.join(RUNDIR, "events.jsonl")
ALLOW = {"hookSpecificOutput": {"hookEventName": "PermissionRequest",
                                "decision": {"behavior": "allow"}}}


def log(kind, obj):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), "kind": kind, **obj}) + "\n")


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        t0 = time.time()
        n = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(n)
        try:
            ev = json.loads(raw)
        except Exception:
            ev = {"raw": raw.decode("utf8", "replace")}
        log("in", {"path": self.path, "len": n, "event": ev})
        name = ev.get("hook_event_name")
        resp = {}
        if name == "PermissionRequest":
            ans = os.path.join(RUNDIR, "answer.json")
            while time.time() - t0 < HOLD and not os.path.exists(ans):
                time.sleep(0.05)
            if os.path.exists(ans):
                resp = json.load(open(ans))
                os.remove(ans)
            else:
                resp = ALLOW
            log("hold", {"waited": time.time() - t0, "from_file": resp is not ALLOW})
        body = json.dumps(resp).encode()
        log("out", {"event": name, "response": resp, "handled_s": time.time() - t0})
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


if os.path.exists(SOCK):
    os.remove(SOCK)
print(f"hook server on unix:{SOCK}, log {LOG}, hold {HOLD}s", flush=True)
Server(SOCK, H).serve_forever()
