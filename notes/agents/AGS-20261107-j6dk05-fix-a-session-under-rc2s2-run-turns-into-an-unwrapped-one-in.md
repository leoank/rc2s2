---
id: AGS-20261107-j6dk05
type: agent_session
schema_version: 1
title: "Fix: a session under rc2s2 run turns into an unwrapped one in the app after the node restarts"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-07T16:34+00:00
finished: null
created_entities: []
edited_entities: []
outcome: pending          # done | failed | aborted | pending
created: 2026-11-07
author: agent:claude
---

## Task

The owner saw a session started with `rc2s2 run` show in the app as wrapped, then after a
while fall back to a session started by hand, losing interrupt, prompt and kill.

## What happened

- The node's log on rogue: wrappers 3655, 4792 and 5892 connected at 07:05–07:07; the node
  restarted at 08:03:55 (a switch), and none said hello again, though all three sessions
  still ran. The wrapper connected once at start and never again, so the new node knew
  their pids only from the registry, as bare sessions.
- Reproduced in `crates/node/tests/run.rs`: a stand-in daemon takes the hello and goes,
  a real one binds the same socket; the session never became wrapped.
- Fix in `run.rs`: one thread says hello, follows the daemon's commands until the link
  closes, and tries again every two seconds for as long as the session runs; keystroke
  notices go over whichever link is current. The no-daemon warning prints once. The test
  passes, as do the workspace tests; the guide's wrapper paragraph says so.
- Sessions started under the old wrapper keep the old behaviour; they need restarting
  under the new binary to benefit.
