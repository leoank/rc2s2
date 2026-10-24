"""RUN-20261019-an0gmm: transcript volume and uuid uniqueness on this machine, counts only.

Walks <claude dir>/projects/*/ the way the node does: a session is <id>.jsonl plus
<id>/subagents/*.jsonl, and a line is kept when it parses, has a `uuid` and is of type
user, assistant or system (crates/node/src/transcript.rs). Prints no content and no ids.
"""
import collections
import datetime
import json
import pathlib
import statistics
import sys

root = pathlib.Path(sys.argv[1]) / "projects"
kept_types = {"user", "assistant", "system"}

sessions = 0
kept = 0
sizes = []
per_day = collections.Counter()
uuid_sessions = collections.defaultdict(set)   # uuid -> sessions it appears in
within_session_repeats = 0                     # a uuid seen twice inside one session's files
for main in root.glob("*/*.jsonl"):
    sessions += 1
    sid = main.stem
    files = [main, *sorted(main.with_suffix("").joinpath("subagents").glob("*.jsonl"))]
    seen = set()
    for f in files:
        with open(f, "rb") as fh:
            for raw in fh:
                try:
                    v = json.loads(raw)
                except ValueError:
                    continue
                u = v.get("uuid") if isinstance(v, dict) else None
                if not u or v.get("type") not in kept_types:
                    continue
                kept += 1
                sizes.append(len(raw))
                if u in seen:
                    within_session_repeats += 1
                seen.add(u)
                uuid_sessions[u].add(sid)
                ts = v.get("timestamp")
                if isinstance(ts, str) and len(ts) >= 10:
                    per_day[ts[:10]] += 1

cross = [n for n in (len(s) for s in uuid_sessions.values()) if n > 1]
days = sorted(per_day)
recent = [per_day[d] for d in days[-30:]]
sizes.sort()
q = lambda p: sizes[min(len(sizes) - 1, int(p * len(sizes)))]
print(f"sessions: {sessions}")
print(f"kept lines: {kept}, distinct uuids: {len(uuid_sessions)}")
print(f"uuid repeated within one session's files: {within_session_repeats}")
print(f"uuids in more than one session: {len(cross)} (max sessions per uuid {max(cross, default=1)})")
print(f"line bytes: mean {statistics.mean(sizes):.0f}, p50 {q(.5)}, p99 {q(.99)}, max {sizes[-1]}")
print(f"active days: {len(days)} from {days[0]} to {days[-1]}")
print(f"kept lines per active day: mean {statistics.mean(per_day.values()):.0f}, max {max(per_day.values())}")
print(f"last 30 active days: mean {statistics.mean(recent):.0f}, max {max(recent)}")
