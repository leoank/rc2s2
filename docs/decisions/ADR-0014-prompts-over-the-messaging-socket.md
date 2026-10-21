# ADR-0014: Prompts go over Claude Code's messaging socket; interrupts over the PTY

Status: accepted
Date: 2026-10-17

## Context
EXP-20261016-rq1405 found Claude Code's cross-session messaging socket: one NDJSON user line from a
same-user process becomes a queued user turn, for sessions started by hand as well. It has
no interrupt. The alternative for prompts was typing them through the wrapper's PTY.

## Decision
Prompts go over the socket; interrupts and kills need the wrapper.

## Consequences
Prompting does not depend on the wrapper. A bypass-mode session holds such prompts for
approval at its terminal (ADR-0022).
