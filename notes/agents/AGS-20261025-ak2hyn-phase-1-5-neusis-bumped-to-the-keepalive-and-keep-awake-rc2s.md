---
id: AGS-20261025-ak2hyn
type: agent_session
schema_version: 1
title: "Phase 1.5: neusis bumped to the keepalive and keep-awake rc2s2, keepAwake on for rogue, ghost001 deployed; rogue's switch is the owner's"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T09:38+00:00
finished: 2026-10-25T09:53+00:00
created_entities:
  - AGS-20261025-ak2hyn
edited_entities: []
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

"Make required changes to neusis, deploy ghost001 and then I can switch rogue as well."

## What happened

- rc2s2 `main` pushed to GitHub at `2487147` (the phase 1.5 items up to 16), neusis's
  rc2s2 input bumped to it, and `keepAwake = true` for rogue in `users/ank/ank.nix` (the
  owner asked for the feature; `claudeUsage` stays off until the credential experiment).
  `darwinConfigurations.rogue.system` builds with the new lock; T0 313/313; committed as
  `ae63bae`.
- `neusis cloud deploy ghost001` from the neusis dev shell: the hub rebuilt on the remote
  builder (spirit) with keepalive on its server and the `ended` notifications; the switch
  outcome is in the closing log line.
- Rogue's switch is the owner's: after it the node runs with `--keep-awake` and the
  keepalive on its hub link.
