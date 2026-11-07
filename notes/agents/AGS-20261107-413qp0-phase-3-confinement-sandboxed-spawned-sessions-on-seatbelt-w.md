---
id: AGS-20261107-413qp0
type: agent_session
schema_version: 1
title: "Phase 3 confinement: sandboxed spawned sessions on seatbelt with a long-lived token"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-07T03:40+00:00
finished: null
created_entities: [EXP-20261107-413q6m, EXP-20261107-413q82, RUN-20261107-438rq0]
edited_entities: []
outcome: pending          # done | failed | aborted | pending
created: 2026-11-07
author: agent:claude
---

## Task
The owner: "Let's now work on confinement" (phase 3, [[PHS-20261106-3702q3]]). Their choices,
asked in one round: a long-lived token from `claude setup-token` instead of waiting for
leases; all egress (no proxy); launch profiles in the node's config, picked in the app; and
inside the sandbox the nix daemon, git identity, the SSH agent, the owner's Claude
CLAUDE.md, skills and commands, and their neusis tools.

## What happened
- Two experiments for the unchecked assumptions: [[EXP-20261107-413q6m]] (the token alone
  runs, resumes, and leaves the keychain alone) and [[EXP-20261107-413q82]] (a stream-json
  session through mxc keeps stdin control, hooks over a unix socket, and what is inside).
- `spikes/07-mxc` gained `src/bin/stream.rs`, a bridge that spawns under seatbelt through
  `mxc_sdk::v1::spawn` and relays stdio (`__KILL__` kills through `MxcProcess::kill`).
- A shell probe under the run's policy, before the token existed: `nix develop -c rustc`
  works through the daemon once `TMPDIR` points inside; git identity through
  `GIT_CONFIG_GLOBAL`; `ssh -T git@github.com` authenticates through the Bitwarden agent
  socket (reachable under `egress: allow`); neusis tools on `PATH`; `~/.ssh/id_ed25519`,
  `~/Documents`, `~/.claude/.credentials.json`, `/tmp/cc-socks`, loopback TCP and writes
  to `~` are refused. coreutils `timeout` is not on macOS.
