"""RUN-20261019-ank8b1: the cost of a unique transcript-uuid index in the hub's events table.

One variant per invocation, on a fresh database, with the schema of
crates/hub/migrations/0001_record.sql and SQLite set as the hub opens it (WAL, synchronous
FULL, sqlx's default page cache of 2 MB unless the variant says otherwise):

  none        the current table: primary key (node_id, seq) only
  text        plus line_uuid TEXT and UNIQUE (node_id, session_id, line_uuid) WHERE NOT NULL
  text-64mb   as text, with a 64 MB page cache
  blob        as text, but the uuid stored as its 16 bytes

Rows imitate a node: random session ids, a new session every 180 lines (this machine's
mean, RUN-20261019-an0gmm), random v4 uuids, a fixed 200-byte payload. Grows to N rows in
transactions of 1,000 and prints the insert rate per million; then, at size, the rate of
single-event transactions (how the hub stores today), the replay of 100,000 lines already
stored under new seqs, and the file size. Synthetic data only.
"""
import os
import sqlite3
import sys
import time
import uuid

variant, path, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
STEP, BATCH, LINES_PER_SESSION = 1_000_000, 1_000, 180
NODE = uuid.uuid4().hex
PAYLOAD = "x" * 200

con = sqlite3.connect(path, isolation_level=None)
con.execute("PRAGMA journal_mode=WAL")
con.execute("PRAGMA synchronous=FULL")
if variant == "text-64mb":
    con.execute("PRAGMA cache_size=-65536")
con.execute("""CREATE TABLE events (
  node_id TEXT NOT NULL, seq INTEGER NOT NULL, session_id TEXT NOT NULL, time_ms INTEGER NOT NULL,
  source TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, line_uuid,
  PRIMARY KEY (node_id, seq))""")
con.execute("CREATE INDEX events_by_session ON events (node_id, session_id, seq)")
if variant != "none":
    con.execute("""CREATE UNIQUE INDEX events_by_line ON events (node_id, session_id, line_uuid)
                   WHERE line_uuid IS NOT NULL""")

SQL = """INSERT INTO events (node_id, seq, session_id, time_ms, source, kind, payload, line_uuid)
         VALUES (?, ?, ?, ?, 'EVENT_SOURCE_TRANSCRIPT', 'assistant', ?, ?) ON CONFLICT DO NOTHING"""
seq = 0
session = str(uuid.uuid4())

def line():
    global seq, session
    seq += 1
    if seq % LINES_PER_SESSION == 0:
        session = str(uuid.uuid4())
    u = uuid.uuid4()
    key = None if variant == "none" else (u.bytes if variant == "blob" else str(u))
    return (NODE, seq, session, seq, PAYLOAD, key)

print(f"variant {variant}")
for step in range(n // STEP):
    t = time.perf_counter()
    for _ in range(STEP // BATCH):
        rows = [line() for _ in range(BATCH)]
        con.execute("BEGIN")
        con.executemany(SQL, rows)
        con.execute("COMMIT")
    rate = STEP / (time.perf_counter() - t)
    print(f"  rows {(step + 1) * STEP:>11,}  batched inserts/s {rate:>9,.0f}")

t = time.perf_counter()
for _ in range(20_000):
    con.execute(SQL, line())
print(f"  single-event transactions/s at size {20_000 / (time.perf_counter() - t):,.0f}")

if variant != "none":
    # Replay: lines the node already sent, renumbered by a restarted node.
    sample = con.execute(
        "SELECT session_id, line_uuid FROM events ORDER BY random() LIMIT 100000").fetchall()
    before = con.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    t = time.perf_counter()
    for s, u in sample:
        seq += 1
        con.execute(SQL, (NODE, seq, s, seq, PAYLOAD, u))
    dt = time.perf_counter() - t
    after = con.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    print(f"  replay of 100,000 stored lines: {100_000 / dt:,.0f}/s, rows added {after - before}")

con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
size = os.path.getsize(path)
print(f"  file bytes {size:,}  per row {size / con.execute('SELECT COUNT(*) FROM events').fetchone()[0]:.0f}")
