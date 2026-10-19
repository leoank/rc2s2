# ADR-0023: Wrappers reach the daemon over a per-user unix socket, and terminal replies are not keystrokes

Status: accepted
Date: 2026-10-19

## Context
`rc2s2 run` must tell the daemon which session it runs and when a person types, and take
interrupts and kills. Terminals answer the TUI's queries on the same input as typing, which
would release every held request on a redraw.

## Decision
JSON lines over a unix socket in a directory only the user can enter, peer uid checked;
the wrapper's child pid maps to the session through Claude Code's registry. `is_human`
counts typing, pastes, arrows and mouse, and ignores cursor, attribute, version, mode and
focus reports.

## Consequences
A session is wrapped only while its wrapper is connected. Kill is wrapped-only.
