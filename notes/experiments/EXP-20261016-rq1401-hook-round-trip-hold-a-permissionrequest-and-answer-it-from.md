---
id: EXP-20261016-rq1401
previous_id: manthan/EXP-0001
type: experiment
schema_version: 1
title: Hook round trip, holding a PermissionRequest and answering it from another process
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [intervention, phase-0, hooks, permissions]
review: approved
---

## Question
Can an HTTP `PermissionRequest` hook hold Claude Code's permission dialog while another
process decides, and what happens when the hold expires? Phase 0 spike 1 of
`code/rc2s2-a-remote-command-and-control-session-server/docs/architecture.md`.

## Hypothesis and predictions
If hooks behave as documented, an `http` hook that delays its response blocks the dialog;
`{"decision": {"behavior": "allow"}}` lets the tool run, and an empty response falls
through to the normal terminal dialog. The per-hook `timeout` bounds the hold. In `-p`
mode without a permission tool the event may not fire at all, which would mean the hold
only applies to interactive sessions.

## Plan
1. A small Python HTTP server that logs every hook event to `events.jsonl` and holds a
   `PermissionRequest` until an answer file appears or the hold expires.
2. A settings file passed with `--settings` that routes every hook event to it.
3. Run `claude` interactively under a PTY in a scratch directory with a prompt that needs
   a Bash permission; answer from a second process; repeat without answering.
4. Repeat in `-p` mode to see whether the event fires there.

## Success criteria
- The event log shows the full `PermissionRequest` input.
- An allow written by the second process makes the tool run with no terminal interaction.
- Letting the hold expire shows the terminal dialog, or the actual behaviour is recorded.
- The maximum usable hold time is known.

## Findings
Spike code: `code/rc2s2-a-remote-command-and-control-session-server/spikes/01-hooks/`
(`hook_server.py`, `pty_driver.py`, settings files). Claude Code 2.1.289.

- **The round trip works.** An `http` hook on `PermissionRequest` receives the full request
  (`session_id`, `transcript_path`, `cwd`, `tool_name`, `tool_input`, `permission_suggestions`)
  and a response of `{"hookSpecificOutput": {"hookEventName": "PermissionRequest",
  "decision": {"behavior": "allow"}}}` runs the tool. Measured hold of 10 s interactive and 8.6 s
  headless, both fine.
- **The terminal dialog is drawn while the hook is pending**, but a keystroke in the terminal is
  queued until the hook returns. The hold therefore blocks the person at the keyboard. The
  daemon must release the hold the moment the PTY wrapper sees any keystroke; then the queued
  keystroke answers the dialog within a fraction of a second. That replaces the idle-time
  heuristic in the architecture doc.
- **Timeout is per hook and exact.** `timeout: 20` cancelled the request at 20.00 s
  (`hook_response` outcome `cancelled`). Interactive: the dialog stays and the terminal can still
  answer. Headless: the tool is denied. Documented default is 600 s; use a large value and rely
  on the keystroke release instead.
- **PermissionRequest fires in `-p` mode** and an allow from the hook runs the tool. This is the
  permission callback for spawned sessions; no MCP permission tool is needed.
  `--include-hook-events` adds `hook_started` and `hook_response` events to the stream.
- **Event order:** UserPromptSubmit, PreToolUse, PermissionRequest (held), then Notification
  `permission_prompt`, PostToolUse, Stop, SessionEnd. The Notification arrives only after the
  PermissionRequest hook returns, so it cannot be the trigger for a push notification; the
  PermissionRequest itself is.
- **SessionStart did not reach a hook configured with `--settings`**; it fires before those
  settings load. The daemon's hooks will be in the user settings file, so this needs a check
  in phase 1, not a redesign.
- Headless streams also expose `rate_limit_event` (five hour and seven day utilisation) and an
  `init` event naming a messaging socket under `/tmp/cc-socks/` with interrupt capabilities.
  Followed up in [[EXP-20261016-rq1402]] and [[EXP-20261016-rq1405]].
- Setup notes: the workspace trust dialog cannot be driven through the PTY with arrow or number
  keys, so the wrapper should start sessions in already trusted directories or the daemon must
  pre-trust them; all `CLAUDE*` environment variables must be stripped from a child session.
- The Opus 5.5 safeguard fallback on a harmless prompt appears as `system/model_refusal_fallback`
  in the stream and as a model-switch dialog in the terminal; the daemon must treat it as an
  event to surface, not an error.

## Runs
- [[RUN-20261016-rq143v]] interactive, allow after a 10 s hold
- [[RUN-20261016-rq14fv]] interactive, hook timeout 20 s, unanswered
- [[RUN-20261016-rq14gh]] headless, timeout cancel measured at 20.00 s
- [[RUN-20261016-rq1443]] headless, empty response leads to denial
- [[RUN-20261016-rq148j]] interactive, terminal answer queued behind the hook
- [[RUN-20261016-rq14zf]] headless, allow after 8 s

## Conclusion
- It is possible to hold the calude code permission dialog using the PermissionRequest hook and asnwer it using a different process.
- There are some edge cases, like handling sessions in not already trusted foldes, as these dialogs cannot be driven by the Pty.

