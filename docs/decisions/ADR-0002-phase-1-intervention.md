# ADR-0002: Phase 1 intervention is allow or deny, answering questions, interrupting and prompting

Status: accepted
Date: 2026-10-16

## Context
What "intervene" must mean in phase 1 (question 2, the owner's answer in `../architecture-questions.md`). Candidates: approve or deny a tool
call, answer a question, send a prompt, interrupt, kill, start a session, edit a pending tool
input.

## Decision
Allow or deny, answer, interrupt, and prompting since it proved cheap. Spawning is phase 2.
Kill came along with the wrapper (ADR-0023).

## Consequences
Phase 1 ends when these work from the phone; editing a tool input and spawning wait.
