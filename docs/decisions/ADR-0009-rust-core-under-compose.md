# ADR-0009: A Rust client core through UniFFI under a Compose app

Status: accepted
Date: 2026-10-16

## Context
Android stack (question 9, the owner's answer in `../architecture-questions.md`): pure Kotlin and Compose, or a shared Rust core.

## Decision
A Rust core exposed through UniFFI, Compose on top, so iOS and integrations reuse it.

## Consequences
More setup in phase 1; the embedded tsnet for the phone lives in the core.
