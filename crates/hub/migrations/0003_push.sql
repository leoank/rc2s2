-- Phase 1 step 5: the UnifiedPush endpoints clients registered (ADR-0026). The hub posts
-- each notification to every endpoint and forgets one that answers 404 or 410.

CREATE TABLE push_endpoints (
  endpoint   TEXT PRIMARY KEY,
  client     TEXT NOT NULL,           -- the name of the token that registered it
  created_ms INTEGER NOT NULL
);
