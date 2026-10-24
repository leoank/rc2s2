---
id: EXP-20261016-rq1403
previous_id: manthan/EXP-0003
type: experiment
schema_version: 1
title: PTY injection for prompt, interrupt and keyboard-activity detection
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [intervention, phase-0, pty, interrupt]
review: approved
---

## Question
Can a wrapper that owns claude's PTY inject a prompt and an Escape mid-turn without
breaking the terminal, and can it observe keyboard activity to drive the permission
hold time? Phase 0 spike 3 plus the second open question.

## Hypothesis and predictions
Bytes written to the PTY master are indistinguishable from typing, so a prompt plus
carriage return submits and Escape interrupts. The wrapper sits between the real
terminal and the PTY, so it sees every keystroke and can keep a last-activity
timestamp for free. Risk: bracketed paste or the input box may need a short delay
between text and Enter.

## Plan
1. Python script using `pty.fork` to run `claude` in a scratch directory, with the
   hook server from [[EXP-20261016-rq1401]] recording events.
2. Wait for the input box, write a prompt and Enter, confirm `UserPromptSubmit` fires.
3. During the next turn write Escape, confirm the turn stops (transcript or hook events).
4. Note any timing needed between writes; note how a human keystroke passing through
   the wrapper is seen.

## Success criteria
- `UserPromptSubmit` fires for the injected prompt.
- The interrupted turn ends without a tool completing and the session stays usable.
- A sketch of the idle-detection logic that the wrapper can implement.

## Findings
Spike code: `spikes/01-hooks/pty_driver.py` (a `pty.fork` driver with scripted steps).

- **Prompt injection works** by writing the text and then a carriage return to the PTY master;
  `UserPromptSubmit` fires with the exact text.
- **Escape interrupts** the running turn immediately. No `Stop` hook fires for an interrupted
  turn, so the daemon must treat "interrupt sent" as the end of the turn itself.
- **After an interrupt the input box is refilled** with the interrupted prompt. The wrapper
  must send Ctrl-U before injecting anything else, or the texts merge into one prompt.
  Sequence that works: Escape, pause, Ctrl-U, text, pause, Enter.
- **Keyboard activity is free**: the wrapper sits between the terminal and the PTY and sees
  every byte the human types. Combined with [[EXP-20261016-rq1401]] (a terminal keystroke is queued behind
  a pending PermissionRequest hook), the rule is: on any human keystroke while a request is
  pending, release the hook with an empty response so the terminal's answer applies at once.
  No idle timer is needed.
- The workspace trust dialog in a fresh directory could not be driven through the PTY with
  arrow or number keys (see [[EXP-20261016-rq1401]]); spawned sessions must start in trusted directories.
- Every `CLAUDE*` environment variable must be stripped before launching a child session.
- Text matching on PTY output is unreliable because the TUI positions text with cursor moves;
  the daemon must not parse the terminal for state, only use hooks and the transcript.

## Runs
- [[RUN-20261016-rq141j]] interrupt while thinking; restored prompt merges with the next
- [[RUN-20261016-rq14xq]] interrupt, Ctrl-U, fresh prompt answered

## Conclusion
- A pty can inject a prompt and an Escape without breaking the terminal.
- A pty can also observe keyboard and drive permission hold time, but there are nuances around parsing terminal state.

