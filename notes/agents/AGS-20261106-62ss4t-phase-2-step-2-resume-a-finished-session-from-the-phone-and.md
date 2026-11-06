---
id: AGS-20261106-62ss4t
type: agent_session
schema_version: 1
title: "Phase 2 step 2: resume a finished session from the phone and session history"
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-11-06T05:32+00:00
finished: 2026-11-06T05:48+00:00
created_entities: [EXP-20261106-62st4b, RUN-20261106-632b3d]
edited_entities: [PHS-20261106-3702ke]
outcome: done             # done | failed | aborted | pending
created: 2026-11-06
author: agent:claude
---

## Task
Start phase 2 step 2 (resume a finished session from the phone, session history) while
the owner switches rogue and tests step 1.4.

## What happened
- [[EXP-20261106-62st4b]] first: a headless `--resume` over stream-json keeps the id,
  the transcript and the context ([[RUN-20261106-632b3d]]), so resume is in place.
- Asked the owner three questions; their answers: one History screen; resume any finished
  session under a spawn root; rows titled by the first prompt with tokens and cost.
  Recorded with the details settled here as ADR-0036.
- Built `History` and `Resume` through proto, hub, node, ctl, core and app; the spawner's
  start path is shared by spawn and resume. Unit tests (spawner resume, the core's row),
  an end-to-end hub test (history with opening and usage, resume, a second resume
  refused). Live with Claude Code 2.1.318 against a scratch hub: history listed, a code
  word kept across a ctl resume and an app resume; on the emulator, History and resuming
  from an ended session's screen.
- Noticed: a resumed session's start time becomes its latest run's (the registry's
  `startedAt`), so it moves to the top of History; written into ADR-0036 as a consequence
  rather than changed.
- Not installed on the owner's phone: History needs the new hub, so it would fail against
  ghost001 until the next bump; the APK is built.
