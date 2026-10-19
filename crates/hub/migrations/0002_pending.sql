-- Phase 1 step 4: permission requests and questions a node is holding, and how each ended.
-- A row is open while closed_ms is NULL. resolution is the Resolution a client sent, as
-- JSON, once the node accepted it.

CREATE TABLE pending (
  node_id    TEXT NOT NULL REFERENCES nodes(id),
  id         TEXT NOT NULL,           -- node-assigned
  session_id TEXT NOT NULL,
  kind       TEXT NOT NULL,
  tool_name  TEXT NOT NULL,
  tool_input TEXT NOT NULL,
  created_ms INTEGER NOT NULL,
  closed_ms  INTEGER,
  reason     TEXT,
  resolution TEXT,
  PRIMARY KEY (node_id, id)
);
CREATE INDEX pending_open ON pending (closed_ms) WHERE closed_ms IS NULL;
