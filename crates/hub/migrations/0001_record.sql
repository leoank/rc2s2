-- Phase 1 step 3: what the hub records from its nodes (architecture.md, "Data model").
-- accounts, credentials, grants, profiles and pending come with the steps that need them.

CREATE TABLE nodes (
  id            TEXT PRIMARY KEY,          -- hub-assigned
  name          TEXT NOT NULL UNIQUE,      -- from the node's token, not its own claim
  version       TEXT NOT NULL,
  first_seen_ms INTEGER NOT NULL,
  last_seen_ms  INTEGER NOT NULL
);

CREATE TABLE sessions (
  node_id         TEXT NOT NULL REFERENCES nodes(id),
  id              TEXT NOT NULL,           -- the harness's session id
  harness         TEXT NOT NULL,
  cwd             TEXT NOT NULL,
  transcript_path TEXT NOT NULL,
  launch          TEXT NOT NULL,
  pid             INTEGER NOT NULL,
  account_label   TEXT NOT NULL,
  status          TEXT NOT NULL,
  started_ms      INTEGER NOT NULL,
  ended_ms        INTEGER,
  PRIMARY KEY (node_id, id)
);

-- Append-only. (node_id, seq) is the node's own numbering, so a replay after a reconnect
-- is ignored instead of stored twice. line_id (a transcript line's uuid) catches the other
-- replay: a restarted node re-reading its transcripts under new seqs. It is unique per
-- session, not per node, since a resumed or forked session repeats lines (EXP-0012).
CREATE TABLE events (
  node_id    TEXT NOT NULL REFERENCES nodes(id),
  seq        INTEGER NOT NULL,
  session_id TEXT NOT NULL,
  time_ms    INTEGER NOT NULL,
  source     TEXT NOT NULL,
  kind       TEXT NOT NULL,
  payload    TEXT NOT NULL,
  line_id    TEXT,
  PRIMARY KEY (node_id, seq)
);
CREATE INDEX events_by_session ON events (node_id, session_id, seq);
CREATE UNIQUE INDEX events_by_line ON events (node_id, session_id, line_id)
  WHERE line_id IS NOT NULL;

CREATE TABLE health (
  node_id TEXT NOT NULL REFERENCES nodes(id),
  time_ms INTEGER NOT NULL,
  metric  TEXT NOT NULL,
  value   REAL NOT NULL
);
CREATE INDEX health_by_node ON health (node_id, time_ms);
