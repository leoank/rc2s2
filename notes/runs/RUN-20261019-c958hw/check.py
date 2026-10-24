"""Summarises RUN-20261019-c958hw: the hub's pending rows, the files and Claude's replies.
Everything here comes from the run's own throwaway sessions."""
import json
import os
import sqlite3
import sys

db, out, tmp, deny_message = sys.argv[1:5]
con = sqlite3.connect(db)
rows = con.execute(
    "SELECT tool_name, tool_input, kind, reason, resolution, closed_ms - created_ms "
    "FROM pending ORDER BY created_ms").fetchall()
print(f"pending requests: {len(rows)}")
for (tool, tool_input, kind, reason, resolution, held_ms), turn in zip(rows, ["allow", "deny", "ignore"]):
    command = json.loads(tool_input).get("command", "").replace(tmp, "$tmp")
    by = json.loads(resolution)["decision"] if resolution else "none"
    print(f"  {turn:6} {kind} {tool} {command!r}: closed {reason} after {held_ms} ms, decision {by}")
for turn, name in [("allow", "allowed.txt"), ("deny", "denied.txt"), ("ignore", "ignored.txt")]:
    exists = os.path.exists(os.path.join(out, name))
    reply = open(os.path.join(tmp, f"{turn}.reply")).read().strip().replace(tmp, "$tmp")
    ctl = open(os.path.join(tmp, f"{turn}.ctl")).read().strip()
    print(f"{turn}: {name} exists {exists}; ctl {ctl or '-'}")
    print(f"  reply: {reply}")
    if turn == "deny":
        print(f"  reply quotes the deny message: {deny_message.split(':')[1].strip()[:20] in reply}")
hooks = con.execute(
    "SELECT kind, COUNT(*) FROM events WHERE source = 'EVENT_SOURCE_HOOK' GROUP BY kind ORDER BY kind").fetchall()
print("hook events: " + ", ".join(f"{k} {n}" for k, n in hooks))
