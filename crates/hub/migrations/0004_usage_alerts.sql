-- Phase 1.5: the rate-limit alerts already pushed (ADR-0032), one per window, reset time and
-- rule, so a node's next sample (or a second node on the same account) does not repeat one.

CREATE TABLE usage_alerts (
  key        TEXT PRIMARY KEY,        -- <window>:<reset minute>:<rule>
  node_id    TEXT NOT NULL,           -- the node whose sample raised it
  created_ms INTEGER NOT NULL
);
