---
id: PHS-20261106-3702q3
type: phase
schema_version: 1
title: "Phase 3: confinement and many networks"
project: PRJ-0001
status: planned
since: null               # the commit it started at: yajna phase start
until: null               # the commit it finished at: yajna phase done
created: 2026-11-06
author: agent:claude
tags: []
---

## Goal
mxc sandbox profiles, embedded tsnet in hub, node and phone, PreToolUse policy rules, a terminal-level adapter, Postgres (`docs/architecture.md`, Phase 3).

## Progress
One item per piece of work, ticked when it lands; add items as the plan grows and keep the
done ones. The project page's *Progress* lists this phase on one line.
- [ ] Phase 3: confinement and many networks: mxc sandbox profiles, embedded tsnet in hub, node and phone, PreToolUse policy rules, a terminal-level adapter, Postgres
- [ ] 1. Confined spawned sessions on seatbelt (the owner's choices of 2026-11-07, [[AGS-20261107-413qp0]]):
  a `claude setup-token` token instead of leases, all egress, launch profiles in the node's
  config picked at spawn, and inside the nix daemon, git identity, the SSH agent, the
  owner's Claude files and tools
  - [x] 1.1 The token alone runs, resumes and leaves the keychain alone ([[EXP-20261107-413q6m]])
  - [x] 1.2 A stream-json session through mxc: stdin control, hooks over a unix socket,
    what is inside, resume; a kill must also end the tool process groups ([[EXP-20261107-413q82]])
  - [x] 1.3 Node: launch profiles from `--profiles`, host and seatbelt through mxc, the
    unix hook socket, the profiles' config dirs scanned beside `~/.claude`, spawn and
    resume under a profile, a hard kill that reaches the tool process groups; the profile
    name through the proto, hub and `ctl spawn --profile` (ADR-0038). Unit and end-to-end
    tests; checked live against a scratch hub with Claude Code 2.1.318: a held request
    allowed from `ctl`, git identity inside and the SSH key hidden, a kill mid-tool with no
    leftovers, a resume in the sandbox that kept the code word, History listing it
    (2026-11-07; [[AGS-20261107-413qp0]])
  - [x] 1.4 The app and the node's profiles: reported to the hub, picked on New session,
    shown on the session; `Session.profile` stored by the hub; `ctl nodes` lists them
    ([[AGS-20261107-7mpyze]])
  - [ ] 1.5 neusis: the profiles option and the token as an agenix secret

## Notes
Migrated from PRJ-0001's Progress by `yajna phase migrate`.
