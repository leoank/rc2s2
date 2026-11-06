---
id: PHS-20261106-3702ke
type: phase
schema_version: 1
title: "Phase 2: launch and own sessions"
project: PRJ-0001
status: active
since: a1296bf15083131872845ddf305e2c9337a90051
until: null               # the commit it finished at: yajna phase done
created: 2026-11-06
author: agent:claude
tags: []
---

## Goal
Start, kill, resume and look back on sessions from the phone: spawn a headless session on a node in a chosen directory, kill any session, resume a finished one by id, browse past sessions with their events and cost, and run them under accounts the hub leases (`docs/architecture.md`, Phase 2). Built in the order of ADR-0034: spawn and kill first under the node's own login, then resume and history, leases last.

## Progress
One item per piece of work, ticked when it lands; add items as the plan grows and keep the
done ones. The project page's *Progress* lists this phase on one line.
- [ ] 1. Spawn and kill under the node's own login, and the keep-awake switch (ADR-0034;
  [[EXP-20261106-37f79k]] checked the stream-json session it rests on)
  - [x] 1.1 Node, hub and `rc2s2 ctl`: `--spawn-root` and `--claude`, the spawner owning
    stdin and stdout (prompt, interrupt, kill, idle and busy from the stream), kill for
    bare sessions, `SetKeepAwake`; spawn roots with the node; unit and end-to-end tests,
    and checked against a scratch hub with the real Claude Code 2.1.318: spawn, interrupt,
    prompt, kill (an interrupt first, or a busy session outlives its closed input until
    the grace kill), a bare `claude -p` killed by SIGTERM, keep-awake on and off with
    `caffeinate` held during a busy turn (2026-11-06; [[AGS-20261106-37f739]])
  - [x] 1.2 The app: a new session from the home screen (node, directory, prompt, model),
    kill in every session's menu, the keep-awake switch on the node's health; checked on
    the emulator against the scratch hub: started on Haiku and opened at once, prompted,
    killed, keep-awake switched on (2026-11-06; [[AGS-20261106-37f739]])
  - [x] 1.3 The neusis node module: `spawnRoots` and the claude path (neusis cdef11d,
    rogue under `~/workspace`, claude from the login shell); rogue switched and ghost001's
    hub deployed by the owner (2026-11-06; [[AGS-20261106-37f739]])
  - [x] 1.4 From the owner's first use: the first prompt optional (the session starts
    idle) and a missing directory created under the root on request, on by default in the
    app (ADR-0035); unit-tested and checked live against a scratch hub; takes effect on
    rogue and ghost001 with the next bump (2026-11-06; [[AGS-20261106-5q9jas]])
- [x] 2. Resume a finished session by id, from the phone; session history: past sessions
  per node with their events and cost. One History screen titled by first prompts with
  tokens and cost, resume in place of any finished session under a spawn root, from its
  screen or `rc2s2 ctl resume` (ADR-0036, the owner's choices; [[EXP-20261106-62st4b]]
  showed a headless resume keeps the id and transcript). Unit and end-to-end tests;
  checked against a scratch hub with the real Claude Code (history, a code word kept
  across two resumes) and on the emulator (History, resume from an ended session's
  screen); takes effect on rogue and ghost001 with the next bump (2026-11-06;
  [[AGS-20261106-62ss4t]])
- [ ] 3. Credential store, accounts with slots, leases to nodes, chosen at spawn time
  ([[EXP-20261016-rq1408]]'s pending refresh run first)

## Notes
Migrated from PRJ-0001's Progress by `yajna phase migrate`.
