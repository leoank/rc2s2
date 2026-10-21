# ADR-0001: Observe sessions through hooks and the transcript, drive them through a PTY wrapper, and spawn headless sessions in phase 2

Status: accepted
Date: 2026-10-16

## Context
How the hub sees and controls a Claude Code session (question 1, the owner's answer in `../architecture-questions.md`). Options: Claude Code
hooks posting to a local daemon, which gate tool calls of any session, including ones started
by hand; spawning headless sessions through the SDK, full control but only for sessions rc2s2
starts; a PTY or tmux wrapper, which works for anything but is brittle to parse; tailing the
transcript JSONL as a backfill. Most sessions are interactive in a terminal and are to be
prompted and controlled from the phone; launching from the phone is wanted too.

## Decision
Hooks plus transcript tailing in phase 1; a PTY wrapper as the input channel for prompts
and interrupts, never parsed for meaning; headless spawning in phase 2. The wrapper is also
the whole of the support for harnesses without hooks (question 3).

## Consequences
Every session on a node is observed, including hand-started ones. Interrupting needs the
wrapper (EXP-20261016-rq1403); prompting turned out not to (ADR-0014).
