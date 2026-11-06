# ADR-0036: Session history on one screen, and finished sessions resumed in place

Status: accepted
Date: 2026-11-06

## Context
Phase 2's second step (ADR-0034): resume a finished session from the phone, and browse
past sessions with their events and cost. [[EXP-20261106-62st4b]] showed that
`claude -p --resume <id>` over stream-json, run in the session's directory, continues the
same session: the same id in its `init` and in the registry, the same transcript file
appended to, the context kept. So a resumed session needs no link to a new one; it is the
old session running again.

## Decision
The owner's choices:
- One History screen, opened from the home screen's top bar: sessions from every node,
  live and ended, newest first, grouped by day, with node filter chips. The home screen
  stays live-only.
- Any finished session can be resumed whose directory is under one of its node's spawn
  roots, sessions started in a terminal included.
- A history row is titled by the session's first prompt, and shows its directory, node,
  start, how long it ran, and its tokens and estimated cost.

Settled by the agent, for the owner to revisit:
- `History` on the control service returns, per session, the stored first few transcript
  `user` lines and the latest daemon `usage` payload; the client core reads the first
  typed prompt from them with the conversation's own rules and prices the usage, so the
  hub keeps storing payloads unparsed. Pages of 50, by start time.
- `Resume` takes the session's directory from the hub's record, since the node may have
  restarted since. The hub refuses a session it records as live; the node refuses one it
  sees live or running, and one it has no transcript of in that directory. A resume runs
  through the spawner as a spawn does, with `--resume` for `--session-id`, so it is
  reported as spawned and takes prompts, interrupts and kill over its input.
- In the app an ended session's screen keeps a prompt bar when it can be resumed: sending
  resumes it with that prompt; *Resume* in the menu resumes it idle.

## Consequences
A session's history is one conversation however often it is resumed. Its start time
becomes that of its latest run, as the registry reports it, so a resumed session moves to
the top of the history and its duration is the latest run's. A session whose directory is
outside the spawn roots, or whose node is offline, can be read but not resumed.
