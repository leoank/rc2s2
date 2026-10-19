# ADR-0013: The PermissionRequest hook is the permission callback, held and released on a keystroke

Status: accepted
Date: 2026-10-17

## Context
How a person decides a permission dialog from another device. EXP-0001 showed a held
PermissionRequest hook blocks the dialog while keystrokes typed at the terminal are queued
until it returns, in interactive and `-p` mode alike; EXP-0005 showed it fires headless too.
Alternatives: an MCP `--permission-prompt-tool` for headless sessions, and an idle timer to
guess when the person at the terminal wants the dialog back.

## Decision
The daemon holds PermissionRequest, reports it to the hub, and answers with the client's
decision; AskUserQuestion is answered the same way through `updatedInput` (EXP-0004). A
wrapped session's hold ends the moment its terminal sees a keystroke; an unwrapped one's
after a fixed hold. No MCP tool, no idle timer.

## Consequences
Deny with a message reaches the model (EXP-0013). Terminal replies must not count as
keystrokes (ADR-0023).
