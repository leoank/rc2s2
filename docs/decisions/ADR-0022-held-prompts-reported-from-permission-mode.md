# ADR-0022: A prompt to a session that bypasses permissions is reported as held, from the session's permission mode

Status: accepted
Date: 2026-10-19

## Context
Claude Code holds socket prompts for approval in bypass mode and sends its receipt only to
a reply socket registered as a session (EXP-20261016-rq1405); catching it means writing a fake entry
into the person's `~/.claude/sessions`.

## Decision
The node reads `permission_mode` from the session's latest hook and, for
`bypassPermissions`, says in the command result's `note` that the prompt waits for approval
at the terminal.

## Consequences
No writes into Claude Code's state. A `crossSessionInbound` setting that overrides the mode
is not seen.
