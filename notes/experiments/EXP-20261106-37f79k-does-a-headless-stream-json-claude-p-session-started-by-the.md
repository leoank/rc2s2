---
id: EXP-20261106-37f79k
type: experiment
schema_version: 1
title: Does a headless stream-json claude -p session started by the daemon show in the session registry, stop its turn on an interrupt control request over stdin, and end cleanly when stdin closes
project: PRJ-0001
status: active            # planned | active | paused | concluded | abandoned
outcome: positive         # null | positive | negative | inconclusive | infra
influenced_by: [EXP-20261016-rq1405]
created: 2026-11-06
author: agent:claude
protocols: []
materials: []
tags: []
review: pending
---

## Question
Phase 2's spawner (ADR-0034) starts `claude -p --input-format stream-json
--output-format stream-json` and owns its stdin. Three things the design rests on were
not checked against the Claude Code in use (2.1.318) by [[EXP-20261016-rq1405]]: does the
daemon's existing session tracking see such a session through the registry, can a turn be
interrupted without a PTY, and does the process end cleanly when the daemon closes stdin?

## Hypothesis and predictions
- H1: the session writes `~/.claude/sessions/<pid>.json` like an interactive one. If not,
  the spawner must register the session itself from the stream's `init`.
- H2: the SDK's `control_request` with subtype `interrupt` ends the running turn and its
  tool. If not, interrupt is SIGINT to the process, or not offered for spawned sessions.
- H3: closing stdin ends the process with exit 0 and removes the registry entry. If not,
  the spawner kills it after a grace period.

## Plan
1. `notes/runs/RUN-20261106-37krk4/run.sh`: spawn one session in
   `target/live-work-spawn-exp` (ignored by the live daemon), start a `sleep 20` through
   Bash, read the registry, send the interrupt, check the turn's `result` and the `sleep`,
   run a second turn, close stdin and check the exit, the registry and the transcript.

## Success criteria
- A registry entry with the given session id while the session runs, gone after exit.
- A `result` within a few seconds of the interrupt, the tool's process gone, the next turn
  normal.
- Exit 0 within a few seconds of closing stdin.

## Runs
One line per run after its link, what was tried and what it showed; `yajna validate`
rejects a bare link here. Metric runs also land in results.tsv.
- [[RUN-20261106-37krk4]] one spawned session on 2.1.318: in the registry at once (its status stuck at busy), interrupted in 10 ms over stdin with the tool killed, exit 0 in 0.8 s on closing stdin; H1 to H3 hold

## Conclusion
Written once, by a human, when status becomes `concluded`.
