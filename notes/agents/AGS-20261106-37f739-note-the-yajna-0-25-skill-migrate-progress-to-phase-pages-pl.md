---
id: AGS-20261106-37f739
type: agent_session
schema_version: 1
title: Note the yajna 0.25 skill, migrate Progress to phase pages, plan phase 2 with the owner and build its first step
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-11-06T02:56+00:00
finished: 2026-11-06T03:23+00:00
created_entities: [PHS-20261106-3701mw, PHS-20261106-3701r1, PHS-20261106-3701nh, PHS-20261106-3702ke, PHS-20261106-3702q3, PHS-20261106-37024q, EXP-20261106-37f79k, RUN-20261106-37krk4]
edited_entities: [PRJ-0001]
outcome: done             # done | failed | aborted | pending
created: 2026-11-06
author: agent:claude
---

## Task
The owner updated the yajna skill (0.25.0, their uncommitted `flake.lock` bump) and asked
for a note of it, then to start building phase 2.

## What happened
- Copied the 0.25.0 skill into `.claude/skills/yajna/SKILL.md` (it adds phases as `PHS`
  pages) and logged it.
- Asked the owner four questions; their answers: migrate Progress to phase pages; spawn
  and kill first under the node's own login, then resume and history, leases last;
  headless stream-json for spawned sessions; the keep-awake switch in step 1. Recorded
  as ADR-0034, with the conventional details settled here listed in it for review:
  `--spawn-root`, the hub naming the session id, the login shell for PATH, the hold of a
  wrapped session, SIGTERM for bare sessions, `node.keep_awake_enabled`.
- `yajna phase migrate PRJ-0001`: six phase pages. Titles that the migration cut with
  "…" were rewritten, and goals written for phases 0 and 2 to 4. Phase 2's checklist
  written and `yajna phase start` run (since a1296bf).
- [[EXP-20261106-37f79k]] first, since step 1 rests on it: a spawned stream-json session
  is in the registry (its status stuck at busy), is interrupted over stdin in 10 ms, and
  exits 0 on closing stdin ([[RUN-20261106-37krk4]]).
- Built 1.1 and 1.2 (protocol, node spawner, hub calls, ctl, core, app), with unit and
  end-to-end tests. Checked live against a scratch hub and node with the real Claude Code
  under `target/live-work-spawn` (ignored by the live daemon) and on the emulator.
- Found live: killing a busy spawned session by closing its input left it running to
  the grace kill (SIGKILL), as claude finishes the turn before reading EOF; kill now
  interrupts first, and exited in under 4 s on recheck.
- Decided without asking: the model list offered (Default, Fable, Opus, Sonnet, Haiku);
  the directory field offers the roots and live sessions' directories under them; the
  switch shows the state set from the app until the next sample reports it; `Serve`
  boxed to keep clippy's variant-size lint quiet once the new flags grew it.
- Not checked: SIGTERM to an interactive TUI session (only a bare `claude -p`); a
  permission request from a spawned session through the app (the scratch node did not
  get the spawned session's hooks, which go to the live daemon on rogue); 1.3, the neusis
  module, waits for the owner's word.
