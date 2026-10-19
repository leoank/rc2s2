# ADR-0010: A thin agent adapter trait with Claude Code as its one implementation

Status: accepted
Date: 2026-10-16

## Context
Agent abstraction (question 10, the owner's answer in `../architecture-questions.md`): an adapter boundary from day one, or hard-coded
Claude Code.

## Decision
A thin trait with two capability levels, structured (hooks or SDK) and terminal (the PTY
wrapper alone), and one implementation.

## Consequences
Not yet extracted in code: phase 1 talks to Claude Code directly in the node crate; the
trait comes before a second harness does.
