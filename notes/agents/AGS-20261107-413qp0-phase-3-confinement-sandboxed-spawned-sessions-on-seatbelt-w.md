---
id: AGS-20261107-413qp0
type: agent_session
schema_version: 1
title: "Phase 3 confinement: sandboxed spawned sessions on seatbelt with a long-lived token"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-07T03:40+00:00
finished: null
created_entities: [EXP-20261107-413q6m, EXP-20261107-413q82, RUN-20261107-438rq0, RUN-20261107-44gt73, RUN-20261107-4dxwmx, RUN-20261107-4dxw07]
edited_entities: [PRJ-0001, PHS-20261106-3702q3]
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
- Both experiments ran once the owner made the token: the token alone works
  ([[RUN-20261107-4dxwmx]]); the sandboxed session works, and a kill through mxc leaves the
  Bash tool's process group running ([[RUN-20261107-4dxw07]]). Claude Code's Bash tool now
  refuses `sleep N; ...` chains, so the first attempt's interrupt and kill hit an idle
  session; the second used a Python sleep. Committed as 0428145 and, by mistake, pushed
  though only a commit was asked.
- The node (item 1.3, ADR-0038): `containment.rs` (profiles from TOML, the seatbelt spawn
  through mxc with its pipes as tokio pipes, the kill of tool process groups), the
  spawner's profile argument, resume finding the profile from the transcript's place,
  the unix hook socket, the registry scan over the profiles' config dirs, and the profile
  name through `Spawn`, `SpawnRequest`, the hub and `ctl spawn --profile`; the core's
  `spawn` still sends no profile (the app's part is item 1.4).
- Decided without asking, recorded in ADR-0038: one config dir per profile; only the
  wrappers' socket file is denied (the first live spawn failed because the scratch state
  dir sat in the wrapper socket's directory, which was denied whole); the hook socket is
  open to any sandbox.
- Live check against a scratch hub on 7790 with real Claude: allowed from `ctl`, git
  identity inside and the key hidden, kill mid-tool with no leftovers, resume in the
  sandbox keeping the code word, History listing the session.
