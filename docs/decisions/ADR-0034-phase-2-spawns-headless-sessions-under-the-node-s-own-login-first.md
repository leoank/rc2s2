# ADR-0034: Phase 2 spawns headless sessions under the node's own login first, leases last

Status: accepted
Date: 2026-11-06

## Context
Phase 2 of `architecture.md` has five parts: spawn a session from the phone, kill any
session, a credential store with leases and grants, resume by id, and session history.
Leases are the largest and riskiest: an encrypted store on the hub, slot logins on the hub
machine, and reading refreshed tokens back from a node. Spawning is usable without them,
since every node already has its owner's own Claude login. The spawned process could be
the headless `claude -p` with stream-json input and output that [[EXP-20261016-rq1405]]
settled on, or the interactive TUI under a PTY the daemon owns, like `rc2s2 run` without a
terminal, which would keep AskUserQuestion and allow attaching later but has nothing to
mirror to and would need new work to attach. The owner also asked to switch keep-awake
(ADR-0031) from the app, a node command of the same kind.

## Decision
The owner's choices: build spawn and kill first under the node's own login, then resume
and history, then the credential store and leases, chosen at spawn time. Spawned sessions
are headless stream-json. The keep-awake switch comes in the first step.

Settled by the agent as conventional details, for the owner to revisit:
- The node spawns only under directories it is started with (`--spawn-root`, repeatable,
  none by default, so spawning is off until configured), checked on the canonical path.
  It reports its roots when it registers; the hub passes them to clients with the node.
- The process starts through the user's login shell (`$SHELL -l -c 'exec "$0" "$@"'`
  with `claude`, or `--claude <path>`, and its arguments), so it gets the PATH and
  environment a terminal would: a daemon under launchd has `/usr/bin:/bin:/usr/sbin:/sbin`.
- The hub picks the session id and the node passes it as `--session-id`, so the client
  can open the session the moment the spawn is answered.
- The node owns the process's stdin and stdout. A prompt is a `user` line on stdin; an
  interrupt is the `control_request` of subtype `interrupt`; kill closes stdin and kills
  the process if it has not exited after a grace period. Idle and busy come from the
  stream (a `user` line sent, a `result` read), since the registry's status of such a
  session stays `busy` ([[EXP-20261106-37f79k]]). The session is found and tailed like any
  other, through the registry and the transcript, and reported as `LAUNCH_SPAWNED`.
- A spawned session's permission dialog is held as long as a wrapped one's
  (`--wrapped-hold-secs`): there is no terminal to fall through to, and a hook that runs
  out denies the tool in print mode.
- Kill works on every session: a wrapped one through its wrapper as before, a spawned one
  as above, a bare one by SIGTERM to its pid from the registry.
- Keep-awake becomes a runtime switch, `SetKeepAwake`, with `--keep-awake` as its state at
  start; `node.keep_awake_enabled` in health says which way it is set.

## Consequences
Spawning is usable on the owner's nodes before any credential work, and leases later only
change which config directory a spawned process gets. A spawned session dies with the
daemon that owns its stdin; resume (the next step) is how it is continued after a daemon
restart. Spawned sessions cannot ask questions (AskUserQuestion does not exist in `-p`)
and cannot be attached at a terminal while they run; `claude --resume <id>` there after
it ends works. `claude` must be on the login shell's PATH, or named with `--claude`.
