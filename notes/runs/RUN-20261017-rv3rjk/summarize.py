"""Summarise `rc2s2 node serve` output by type and kind; never prints payloads or paths."""
import collections
import json
import sys

out, hook_status, hook_reply = sys.argv[1], sys.argv[2], sys.argv[3]
kinds, sessions, seqs, health = collections.Counter(), {}, [], {}
for line in open(out):
    o = json.loads(line)
    if o["type"] == "event":
        kinds[(o["source"], o["kind"])] += 1
        seqs.append(o["seq"])
    elif o["type"] == "session":
        sessions[o["id"]] = o
    elif o["type"] == "health":
        health = o["metrics"]

print(f"synthetic hook: http {hook_status}, reply {hook_reply}")
print(f"events: {len(seqs)}, seq strictly increasing: {seqs == sorted(set(seqs))}")
for (source, kind), n in sorted(kinds.items()):
    print(f"  {source:<26} {kind:<14} {n}")
print(f"sessions: {len(sessions)}")
for sid, s in sorted(sessions.items()):
    print(f"  {sid[:8]}  pid {s['pid']:<6} {s['status']:<27} transcript {'yes' if s['transcript_path'] else 'no'}"
          f"  ended {'yes' if s['ended_ms'] else 'no'}")
print("health:")
for name, value in sorted(health.items()):
    print(f"  {name:<20} {value:.1f}")
