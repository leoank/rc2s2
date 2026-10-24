---
id: EXP-20261017-rwda0a
previous_id: manthan/EXP-0010
type: experiment
schema_version: 1
title: Does the observe-only node daemon see what a real Claude Code session does?
project: PRJ-0001
status: active            # planned | active | paused | concluded | abandoned
outcome: positive             # null | positive | negative | inconclusive | infra
influenced_by: [EXP-20261016-rq1401, EXP-20261016-rq1402, EXP-20261016-rq1405, EXP-20261016-rq1409]
created: 2026-10-17
author: agent:claude
protocols: []
materials: []
tags: [observation, phase-1, node, hooks, transcript, health]
review: approved
---

## Question
Does the phase 1 node daemon, observing only, see what a real Claude Code session does on
its machine: every hook event, every message line of the transcript under the hook's
session id, usage counted once per API message, and the sessions in the registry with
their pid and status? This is the check behind step 2 of phase 1 in the *Progress*
checklist on [[PRJ-0001]]; the facts it relies on were established in [[EXP-20261016-rq1401]] (hooks),
[[EXP-20261016-rq1402]] (transcript shapes and usage), [[EXP-20261016-rq1405]] (session registry) and
[[EXP-20261016-rq1409]] (GPU).

## Hypothesis and predictions
If the phase 0 findings carry over into `crates/node`, then for one real session: each hook
Claude fires reaches the endpoint, in order, answered `{}` without changing Claude's
behaviour; the transcript path from the hook input leads the tailer to lines that all carry
that session id; summing usage once per `message.id` equals the same sum taken straight
from the file, with fewer messages than assistant lines; the registry yields the session's
pid and busy status, and the session ends when it leaves the registry. Against the live
machine, the registry scan finds the sessions already running, with their transcripts
backfilled. If not, the mismatch names the finding that did not carry over.

## Plan
1. Run the daemon read-only against this machine's live state for a few seconds, post one
   synthetic hook, and summarise its output by kind without printing content.
2. Run one throwaway `claude -p` turn on haiku in a scratch directory, its hooks pointed at
   the daemon with `--settings` from `rc2s2 node hooks`, since `~/.claude/settings.json` is
   managed read-only by neusis; check the daemon's output against the transcript file.
3. Later, with the hooks in the real settings file (the home-manager module of step 6):
   SessionStart, PermissionRequest, Notification and SubagentStop from an interactive
   session.

## Success criteria
- Every hook event of the turn arrives, in order, under one session id, answered `{}`.
- Every kept transcript line carries that session id; nothing from the attachment or
  session-state families is kept.
- Usage totals from the daemon equal a once-per-`message.id` sum over the file.
- The registry gives pid and status, and leaving it ends the session.
- Live sessions found by the scan appear with pid, status and a transcript; health includes
  the GPU on macOS.

## Runs
- [[RUN-20261017-rv3rjk]]: live machine state, read only (plan 1).
- [[RUN-20261017-q4m07f]]: hooks from a throwaway `claude -p` turn (plan 2).

## Findings
- All five hook events of a `-p` turn with a Bash call (UserPromptSubmit, PreToolUse,
  PostToolUse, Stop, SessionEnd) arrived in order under one session id
  ([[RUN-20261017-q4m07f]]). SessionStart does not fire from `--settings`, as [[EXP-20261016-rq1401]]
  found.
- The six kept transcript lines all carry the hook's session id; three assistant lines with
  two message ids sum to two messages and 151 output tokens in the daemon and in the file.
- A `-p` session is in the registry too, with pid and busy status, so the registry covers
  headless sessions, not only interactive ones as [[EXP-20261016-rq1405]] described.
- On the live machine the scan found all four running sessions with pid, status and
  transcript, backfilled about 2,500 kept lines, and health included the GPU
  ([[RUN-20261017-rv3rjk]]). The machine had five `claude` processes for four registry
  sessions; `claude.processes` counts processes, not sessions.

## Conclusion
The node daemon sees what a real claude code session sees. (Within the scope of the daemon).
