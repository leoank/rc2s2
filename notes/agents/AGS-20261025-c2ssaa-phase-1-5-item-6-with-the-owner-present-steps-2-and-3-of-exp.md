---
id: AGS-20261025-c2ssaa
type: agent_session
schema_version: 1
title: "Phase 1.5 item 6 with the owner present: steps 2 and 3 of EXP-20261025-25f6vm on the real credential, then the poller on for rogue"
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-10-25T11:00+00:00
finished: null
created_entities:
  - AGS-20261025-c2ssaa
  - RUN-20261025-c35jpc
  - RUN-20261025-c5xjbh
edited_entities:
  - PRJ-0001
  - EXP-20261025-25f6vm
outcome: pending          # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

Step 6 of phase 1.5, the part [[AGS-20261025-92ty85]] left for the owner: steps 2 and 3 of
[[EXP-20261025-25f6vm]] on the real credential, then `claudeUsage` on for rogue in neusis.

## What happened

- Read the real item's attributes and, from the terminal, the token's hash and expiry only
  (12:59:07Z); no prompt. `claude -p "/usage"` works headless, so it is the comparison.
- **[[RUN-20261025-c35jpc]]** (step 2): four polls from a node with no terminal, 40% and 40%
  as `/usage` showed; item, token and login unchanged; the token in no log.
- **[[RUN-20261025-c5xjbh]]** (step 3): the same script left polling until 13:15Z, across the
  expiry. The first attempt failed at once: GNU `date` in the dev shell has no `-j`; the
  duration is now computed in Python.
- **The Fable week, asked for by the owner.** `/usage` shows a third window, "Current week
  (Fable)", that the response carries only as a `weekly_scoped` entry in `limits` with
  `scope.model.display_name` "Fable". `health::usage_windows` now reads the two windows and
  that entry for both the live poll and the cached value, reported as
  `usage.seven_day_fable_pct`; `usage::utilisation` is gone into it. The app charts "Claude
  7-day Fable limit". Node tests pass (a new one on the real `limits` shape); the APK
  builds. Not checked on the emulator. Decided without asking: the metric name, and
  matching Fable by name rather than charting every scoped week.
