---
id: AGS-20261110-g5mg21
type: agent_session
schema_version: 1
title: "Rethink agent credentials around claude setup-token: supersede the leased slots of ADR-0018 and plan the experiments the new design rests on"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-10T14:43+00:00
finished: 2026-11-10T15:05+00:00
created_entities: [EXP-20261110-g5mh7g, EXP-20261110-g5mht5, EXP-20261110-g5mh2z, EXP-20261110-g5mh89]
edited_entities: [PRJ-0001, PHS-20261106-3702ke]
outcome: done             # done | failed | aborted | pending
created: 2026-11-10
author: agent:claude
---

## Task
The owner asked whether the year-long `claude setup-token` token, shareable across
sessions, makes the credential design simpler now and once a proxy controls access; on
the agent's yes they asked for its recommendation to be carried out: a decision record
superseding ADR-0018, the architecture and phase 2 step 3 updated, and experiments
planned for the assumptions.

## What happened
- ADR-0039 written: an account is a label with one setup-token per node, held encrypted
  by the hub and sent with a spawn; no slots, leases, read-back or refresh; expiry alerts;
  the egress proxy later adds the token so a sandbox holds a dummy. ADR-0018 marked
  superseded. The leased-slot reasoning stays in the architecture as why a refreshing
  login is never copied.
- `docs/architecture.md`: Credentials rewritten; the spawn flow, node service, rate
  limits, data model (no `grants` table: a token for an account and node is the grant),
  the sandbox secrets policy and phase 2 updated.
- [[PHS-20261106-3702ke]] step 3 replanned in four items; the project page's lesson on
  copying credentials notes the setup-token exception.
- Planned [[EXP-20261110-g5mh7g]] (one token from two machines at once),
  [[EXP-20261110-g5mht5]] (usage endpoint with a token), [[EXP-20261110-g5mh2z]] (revoking
  one token) and [[EXP-20261110-g5mh89]] (how a sandbox reaches a token-adding proxy).
  None run: each needs the owner to make tokens in a browser.
- Decided without asking, marked in the ADR for the owner: one token per node and account
  (the owner had not picked between per node and per profile); the profile `token_file`
  stays until the store replaces it; a token's creation date is entered, not read.
- The owner then reshaped it: named tokens on the hub, picked per session, a default per
  node managed from the hub and the app, the token on the node until the hub side exists,
  dates inferred from when a token is added with a reminder a month ahead and settable by
  hand. Asked three questions; answers: a node's default applies to every spawn, host
  included; the hub gets the tokens from neusis secrets (so no encrypted table and no
  secret through a client); any node may use any token now, a list of nodes per token
  later. ADR-0039 rewritten (and renamed) to match, before it was ever committed; the
  architecture and step 3 of [[PHS-20261106-3702ke]] (now five items) follow it.
- Settled without asking, in the ADR: `--claude-tokens` copies the peer token files'
  format; a new fingerprint under a name is a renewal; "the node's own login" stays a
  choice in the picker for host sessions.
