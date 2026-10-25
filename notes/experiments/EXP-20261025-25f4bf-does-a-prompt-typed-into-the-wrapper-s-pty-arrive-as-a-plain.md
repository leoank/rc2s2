---
id: EXP-20261025-25f4bf
type: experiment
schema_version: 1
title: Does a prompt typed into the wrapper's PTY arrive as a plain user turn, and does one typed during a running turn queue rather than interrupt?
project: PRJ-0001
status: planned           # planned | active | paused | concluded | abandoned
outcome: null             # null | positive | negative | inconclusive | infra
influenced_by: [EXP-20261016-rq1403, EXP-20261019-fgm60e]
created: 2026-10-25
author: agent:claude
protocols: []
materials: []
tags: [prompts, pty, wrapper, phase-1.5]
---

## Question
ADR-0027 moves a wrapped session's prompt from the messaging socket to the PTY, so the
model sees a typed prompt and not a peer message. The wrapper already types a follow-up
after an interrupt ([[EXP-20261016-rq1403]], [[RUN-20261019-mvz0r3]]), always into an idle input box.
Two things are unchecked: that a prompt typed while the session is idle reaches the model
with nothing added (no banner, the usual UserPromptSubmit, the usual permission mode),
and what happens to text typed while a turn is running: Claude Code queues typed input
behind the turn in recent versions, but whether the wrapper's text and Enter arrive as one
queued prompt, get split, or are lost has not been observed under rc2s2.

## Hypothesis and predictions
H1: a prompt typed into the PTY of an idle wrapped session appears in the transcript as a
plain user line equal to the text, UserPromptSubmit fires with that prompt, and the reply
answers it. H2: text and Enter typed during a running turn are queued by Claude Code and
become the next user turn, unchanged, after the running turn's Stop, with no interrupt.
If H2 is false we expect the prompt to be lost, truncated, or merged with the input box's
content, and the daemon must then wait for Stop before typing (ADR-0027, consequences).

## Plan
1. A scratch hub and node as in [[RUN-20261025-166zqd]], a haiku session under `rc2s2 run`
   in a scratch directory under `target/live-work-*`, and `rc2s2 ctl prompt` (or the app)
   against the daemon's new PTY path.
2. Idle: prompt "Reply with just the word pelican." Record the transcript's user line, the
   UserPromptSubmit payload, and the reply.
3. Busy: start a count to 3000 at the terminal, prompt "Reply with just the word heron."
   from the client after 3 s, and record whether the count finishes, whether a Stop fires
   before the follow-up's UserPromptSubmit, and what the follow-up's user line contains.
4. Bypass mode: the same idle prompt in a `--dangerously-skip-permissions` session, to see
   that nothing is held (ADR-0022 applied to the socket only).

## Success criteria
- The idle prompt's transcript line is exactly the text, with no peer banner, and the
  reply answers it.
- The busy prompt arrives as its own user turn after the running turn, unchanged; or the
  failure mode is recorded and the daemon's rule (wait for Stop) is chosen from it.
- The bypass-mode prompt is answered without a hold.

## Runs
One line per run after its link, what was tried and what it showed; `yajna validate`
rejects a bare link here. Metric runs also land in results.tsv.

## Conclusion
Written once, by a human, when status becomes `concluded`.
