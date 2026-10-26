---
id: AGS-20261025-c2ssaa
type: agent_session
schema_version: 1
title: "Phase 1.5 item 6 with the owner present: steps 2 and 3 of EXP-20261025-25f6vm on the real credential, then the poller on for rogue"
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-10-25T11:00+00:00
finished: 2026-10-25T23:08+00:00
created_entities:
  - AGS-20261025-c2ssaa
  - RUN-20261025-c35jpc
  - RUN-20261025-c5xjbh
edited_entities:
  - PRJ-0001
  - EXP-20261025-25f6vm
outcome: done          # done | failed | aborted | pending
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
- **Step 3 ended at 13:38** (the machine slept through the loop's end). Claude Code
  refreshed the token itself at 12:55:36Z and the login stayed; but from 11:28 the
  endpoint answered 429 to every once-a-minute poll for half an hour. The machine also slept
  twice with the network gone around the wakes. A 70 s rerun at 13:42 showed the daemon
  reading the new token (`after-refresh.txt`).
- **Asked the owner:** five minutes with backoff, and the flag on for rogue after the fix.
  The poller now waits `Retry-After`, else doubles up to 30 minutes on a 429; default
  interval 300 s (commit `dfc318b`), with tests against a real 429.
- **Neusis, uncommitted:** `claudeUsage = true` for rogue and the option's description;
  `nix flake check --no-build` and the daemon's build pass with the rc2s2 input overridden
  to this checkout. It needs rc2s2 pushed and the input bumped, or rogue would run the old
  once-a-minute poller.
- **Asked to push and bump:** rc2s2 pushed (`2487147..d52e5a4`); `nix flake update rc2s2` in
  neusis and the change committed with the lock as `b309df4` (not pushed). On exactly what
  was staged, `nix flake check --no-build` passed, rogue's launchd arguments carry
  `--claude-usage`, and the daemon builds with a 300 s default. Item 6 ticked; the switch on
  rogue and the hub rebuild on ghost001 are the owner's.
- **Proposed conclusion for [[EXP-20261025-25f6vm]]:** positive for H1 and H2 (read only,
  no prompt, the login undisturbed across a refresh); H3 untested; the endpoint's rate
  limit is the finding that changed the design.
