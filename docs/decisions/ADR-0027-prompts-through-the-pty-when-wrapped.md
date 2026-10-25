# ADR-0027: A prompt is typed into the PTY when the session runs under the wrapper; the socket stays for the rest

Status: accepted (amends ADR-0014)
Date: 2026-10-25

## Context
ADR-0014 sent prompts over Claude Code's messaging socket so prompting would not depend on
the wrapper. Using the app showed the cost: Claude Code delivers a socket message as a
peer message, wrapped in "Another Claude session sent a message" with a paragraph telling
the model to treat it as a teammate of limited trust, never as the user's approval
([[RUN-20261025-166zqd]]). The app hides the banner, but the model still answers a peer,
not its user. The owner wants a prompt from the phone to be indistinguishable from one
typed at the terminal. The alternatives were the PTY for every prompt (so sessions started
by plain `claude` could not be prompted), the PTY only when the wrapper is there, or a
search for a socket message shape that Claude Code shows as a plain user turn.

## Decision
When the session runs under `rc2s2 run`, the daemon types the prompt into the PTY as the
wrapper already does for a follow-up after an interrupt (text, then Enter), so Claude Code
sees a typed prompt. Sessions without the wrapper keep the socket, and the app says before
sending that the model will see the prompt as a message from another session.
[[EXP-20261025-25f4bf]] checks the two things the design rests on: that a typed prompt
produces the usual UserPromptSubmit and transcript line with nothing added, and that one
typed while a turn runs is queued by Claude Code rather than interrupting it.

## Consequences
A wrapped session's prompt is the user's own words, with the usual hooks and permission
mode; bypass-mode holding (ADR-0022) no longer applies to it. The two paths differ in
timing: the PTY path needs the input box to be free, which the wrapper cannot see
(terminal text is not parsed, ADR-0001); if the experiment finds typed text lost during a
turn, the daemon waits for the turn's Stop before typing. The socket stays the fallback,
with the banner, for sessions rc2s2 did not start.
