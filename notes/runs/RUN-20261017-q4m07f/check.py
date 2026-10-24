"""Checks the daemon's output for the one throwaway session that posted hooks: hook order,
transcript lines under the hook's session id, usage once per message id against the file."""
import collections
import json
import sys

lines = [json.loads(l) for l in open(sys.argv[1])]
hooks = [o for o in lines if o["type"] == "event" and o["source"] == "EVENT_SOURCE_HOOK"]
ids = {h["session_id"] for h in hooks}
assert len(ids) == 1, f"expected one hook session, got {len(ids)}"
sid = ids.pop()
mine = [o for o in lines if o.get("session_id") == sid or o.get("id") == sid]

print(f"hook events, in seq order: {[h['kind'] + ('(' + h['payload']['tool_name'] + ')' if 'tool_name' in h['payload'] else '') for h in hooks]}")
transcript = [o for o in mine if o["type"] == "event" and o["source"] == "EVENT_SOURCE_TRANSCRIPT"]
print(f"transcript lines kept: {dict(collections.Counter(o['kind'] for o in transcript))}")
print(f"every kept line has the hook's session id: {all(o['payload'].get('sessionId') == sid for o in transcript)}")

paths = {h["payload"]["transcript_path"] for h in hooks if "transcript_path" in h["payload"]}
assert len(paths) == 1
by_id = {}
assistant_lines = 0
for raw in open(paths.pop()):
    o = json.loads(raw)
    if o.get("type") == "assistant":
        assistant_lines += 1
        by_id[o["message"]["id"]] = o["message"]["usage"]
usage = [o for o in mine if o["type"] == "event" and o["kind"] == "usage"]
totals = usage[-1]["payload"] if usage else {}
daemon_out = sum(m["output_tokens"] for m in totals.values())
daemon_msgs = sum(m["messages"] for m in totals.values())
file_out = sum(u["output_tokens"] for u in by_id.values())
print(f"file: {assistant_lines} assistant lines, {len(by_id)} message ids, {file_out} output tokens once per id")
print(f"daemon: {daemon_msgs} messages, {daemon_out} output tokens; match: {(daemon_msgs, daemon_out) == (len(by_id), file_out)}")

for o in mine:
    if o["type"] == "session":
        print(f"session update: pid {o['pid']}, {o['status']}, {o['launch']}, ended {'yes' if o['ended_ms'] else 'no'}")
