# ADR-0038: Sandboxed spawns on seatbelt, under launch profiles from the node's config, with a long-lived token

Status: accepted
Date: 2026-11-07

## Context
Phase 3 starts with confinement: sessions rc2s2 starts should run with less than the
owner's privileges (ADR-0017, "Sandboxed sessions" in the architecture). The design there
hands each sandbox a leased slot credential, which phase 2 step 3 has not built. The owner's
own keychain login cannot be shared with a sandbox: a second store of one grant is revoked
at the first refresh ([[RUN-20261017-dssg3d]]). The node runs only on rogue, a Mac, so
seatbelt is the backend that matters now.

The owner chose, on 2026-11-07 ([[AGS-20261107-413qp0]]): a token from
`claude setup-token` instead of waiting for leases; all egress rather than a proxy
allowlist; launch profiles defined in the node's config and picked in the app; and inside,
the nix daemon, their git identity, the SSH agent, their Claude `CLAUDE.md`, skills and
commands, and their neusis tools.

[[EXP-20261107-413q6m]] showed the token alone runs headless turns and resumes them in an
empty config directory, reports rate limits, and leaves the keychain and the owner's login
alone. [[EXP-20261107-413q82]] showed a stream-json session through mxc's seatbelt backend
keeps stdin control (prompt, interrupt, end on closed input), delivers every hook,
SessionStart included, as command hooks over a unix socket, runs `nix develop`, git and
SSH inside, refuses the owner's keys, home and the other sessions' sockets, and keeps its
registry entry and transcript in its own config directory, where `--resume` finds it; and
that a kill through mxc leaves the running tool's process group behind.

## Decision
- **Profiles.** `rc2s2 node serve --profiles <file.toml>` names launch profiles; a spawn
  names one (`Spawn.profile`, `SpawnRequest.profile`, `rc2s2 ctl spawn --profile`), and the
  empty name is the host, as before. A profile has a backend (`host` or `seatbelt`), a
  description, and for a sandbox a `state_dir`, a `token_file`, `path`, `readonly`,
  `readwrite` and `denied` paths, `claude_files` linked from `~/.claude`, and `env`. `~/`
  is the node's home. The node refuses a file it could not run (seatbelt off macOS, a
  sandbox without state directory or token, two profiles sharing a state directory) and a
  spawn naming a profile it lacks.
- **The sandbox.** mxc's seatbelt backend, through `mxc_sdk::v1::spawn` with the stdio
  pipes taken as tokio pipes. Read-write: the state directory (`claude/` as
  `CLAUDE_CONFIG_DIR`, `tmp/` as `TMPDIR` and `CLAUDE_CODE_TMPDIR`, `home/` as `HOME`) and
  the working directory. Read-only: `/nix/store`, the profile's paths, the linked Claude
  files. Denied: `/private/tmp/cc-socks` and the node's wrapper socket, plus the profile's
  list. Network: egress allowed, no inbound, no loopback TCP. Environment: only what the
  node sets (`PATH` from the profile plus the system directories, `USER`, `LANG`, `TERM`,
  the token as `CLAUDE_CODE_OAUTH_TOKEN`, read from its file at each start) and the
  profile's `env`. claude is found on the profile's `PATH`; no login shell runs inside.
- **Hooks** come back over a unix socket the node binds when a profile is sandboxed
  (`--hook-socket`, default `~/.local/state/rc2s2/hook.sock`), passed with `--settings` as
  command hooks running `/usr/bin/curl --unix-socket`. The socket serves the same router as
  the TCP endpoint, so holds, the app and `ctl` work unchanged.
- **Watching.** The node scans the registry of every sandboxed profile's config directory
  beside its own, and tails transcripts where the registry or the spawn says they are.
  A resume runs under the profile whose config directory holds the transcript, so a
  sandboxed session stays sandboxed.
- **Kill.** A hard kill (the session outlived its closed input) also kills the process
  groups of the session's descendants, other than the node's own, for host and sandboxed
  sessions alike.

Settled by the agent, for the owner to revisit:
- One config directory per profile, shared by its sessions, rather than one per session:
  the registry, History and resume need a fixed place, and a sandbox can read its sibling
  sessions' transcripts.
- Only the wrappers' socket file is denied, not its directory: a state directory under
  that directory would otherwise be unwritable.
- The hook socket is reachable from any sandbox and any local process of the user, as the
  TCP endpoint already is; a sandbox can post hooks claiming another session's id.
- The token's file is the owner's to create; the node never writes it.

## Alternatives considered
- Wait for leases (phase 2 step 3): the end-state design, but confinement would wait on
  the credential store; slots can replace the token per profile later.
- The keychain inside the sandbox: needs read-write on `~/Library/Keychains` and the real
  `HOME`, and puts the owner's own grant in reach of the session.
- A proxy allowlist for egress: tighter, but cooperative on seatbelt (`HTTP_PROXY`), and
  tools that ignore the proxy fail; a profile can gain it later.
- Profiles in the hub, edited from the app: needs an editor and sync to nodes for little
  gain while one owner runs one node.
- A per-session config directory: stricter, but History, resume and the registry scan
  would need to find every one.

## Consequences
- The app and `ctl` must learn the node's profile names (item 1.4 of phase 3); until then
  `ctl spawn --profile` is the way in.
- neusis must write the profiles file and provide the token as a secret (item 1.5).
- With the SSH agent socket reachable, a sandboxed session can push with the owner's keys;
  a profile that leaves `SSH_AUTH_SOCK` out cannot.
- `mxc-sdk` 1.0.0 and its dependency tree enter the node's build on macOS.
- A killed sandboxed session leaves its registry record, as on the host; the scan ignores
  records whose pid is gone.
