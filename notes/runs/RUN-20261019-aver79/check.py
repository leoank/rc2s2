"""Summarises the hub database of RUN-20261019-72r8bn and RUN-20261019-aver79 by counts only; prints no content."""
import json
import sqlite3
import sys

db, per_phase, registrations = sys.argv[1], int(sys.argv[2]), sys.argv[3]
con = sqlite3.connect(db)
q = lambda sql, *a: con.execute(sql, a).fetchall()

print(f"nodes: {q('SELECT COUNT(*) FROM nodes')[0][0]}, registrations logged by the hub: {registrations}")

marks = {}
for seq, payload in q("SELECT seq, payload FROM events WHERE session_id = 'exp-0011' ORDER BY seq"):
    m = json.loads(payload).get("mark")
    if m:
        marks.setdefault(m, []).append(seq)
ok = True
for phase in ["before", "during", "after", "restart"]:
    counts = [len(marks.get(f"{phase}-{i}", [])) for i in range(1, per_phase + 1)]
    ok &= counts == [1] * per_phase
    print(f"marked hooks {phase:8} stored once each: {counts == [1] * per_phase} (copies {counts})")
first = [s for m, ss in marks.items() if not m.startswith("restart") for s in ss]
second = [s for m, ss in marks.items() if m.startswith("restart") for s in ss]
increasing = bool(first and second) and min(second) > max(first)
print(f"seqs after the node restart above every earlier mark: {increasing}")
mark_order = [m for m, _ in sorted(((m, s) for m, ss in marks.items() for s in ss), key=lambda x: x[1])]
print(f"marks stored in posting order: {mark_order == sorted(mark_order, key=lambda m: (['before','during','after','restart'].index(m.rsplit('-',1)[0]), int(m.rsplit('-',1)[1])))}")

total = q("SELECT COUNT(*) FROM events")[0][0]
print(f"events: {total}")
for source, kind, n in q("SELECT source, kind, COUNT(*) FROM events GROUP BY 1, 2 ORDER BY 3 DESC"):
    print(f"  {source:8} {kind:24} {n}")
dups = q("""SELECT kind, COUNT(*), SUM(c) FROM (
              SELECT kind, payload, COUNT(*) c FROM events GROUP BY session_id, kind, payload HAVING c > 1)
            GROUP BY kind ORDER BY 2 DESC""")
print("identical payloads stored under more than one seq (kind, payloads, rows):")
for kind, payloads, rows in dups or [("none", 0, 0)]:
    print(f"  {kind:24} {payloads} {rows}")

s = q("SELECT COUNT(*), SUM(ended_ms IS NULL), SUM(pid > 0), SUM(transcript_path != '') FROM sessions WHERE id != 'exp-0011'")[0]
print(f"sessions from the machine: {s[0]} (open {s[1]}, with pid {s[2]}, with transcript {s[3]})")
h = q("SELECT COUNT(DISTINCT time_ms), COUNT(DISTINCT metric), COUNT(*) FROM health")[0]
gpu = q("SELECT COUNT(*) FROM health WHERE metric LIKE 'gpu%'")[0][0]
print(f"health: {h[0]} samples, {h[1]} metrics, {h[2]} rows, gpu rows {gpu}")
print(f"all marked hooks exactly once: {ok}")
