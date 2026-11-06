# ADR-0037: Spawned sessions carry the node's hooks, and a spawn picks a permission mode

Status: accepted
Date: 2026-11-06

## Context
The owner spawned a session from the phone on rogue and every tool that needed approval
failed at once ("This command requires approval"); nothing reached the app. Rogue's
`~/.claude/settings.json` had lost rc2s2's hook block (it holds empty `PreToolUse` and
`PostToolUse` lists, rewritten that day). ADR-0034 assumed the user's settings carry the
hooks, as phase 1 requires; a spawned session then had no PermissionRequest hook, and
`claude -p` has no terminal to fall back to, so it refuses the tool.

[[EXP-20261106-qt94y4]] checked, on Claude Code 2.1.318: with no hook the tool is refused;
a PermissionRequest hook passed only with `--settings` is asked, and its allow runs the tool
and its deny refuses it; the same hook also in a project's settings is asked once, not
twice; `--dangerously-skip-permissions` and `--permission-mode bypassPermissions` both
start the session in bypass mode, and the hook is not asked.

The owner also asked for a way to start a session with flags such as
`--dangerously-skip-permissions`.

## Decision
- The spawner passes the node's own hooks block (what `rc2s2 node hooks` prints by default)
  to every session it starts or resumes, as `--settings`. A spawned session reaches the node
  whatever the user's settings file says; with the block also there, Claude Code asks each
  hook once.
- `Spawn` and `SpawnRequest` gain `permission_mode`: one of the values claude's
  `--permission-mode` lists, passed through; empty leaves the user's default. The node
  refuses any other value, so no other flag can be passed this way. The app's New session
  dialog offers Ask (the default), Accept edits, Plan and Bypass, and says in red what
  Bypass does; `rc2s2 ctl spawn` takes `--permission-mode`.

Settled by the agent, for the owner to revisit:
- A permission mode, not free-form extra flags: it covers the request, and arbitrary flags
  from a phone would reach a process on the node unchecked (`--mcp-config`, `--add-dir`,
  `--settings`).
- No node-side switch to forbid bypass: a client that can spawn can already allow every
  request.
- Resume does not take a mode: a resumed session runs in the user's default mode, now with
  its permission requests reaching the app.

## Alternatives considered
- Restore the hooks in rogue's settings file and change nothing: fixes rogue once, and the
  next rewrite of the file breaks spawned sessions again, silently.
- The SDK's permission callback (`--permission-prompt-tool stdio`, `can_use_tool` control
  requests on the session's stdout): the right tool for a process the node owns, but a
  second path into the held requests next to the hook, for no gain the hook does not give.
- Free-form extra flags on a spawn: see above.

## Consequences
- The node must listen on the address its hooks block names (`--hook-addr`), which it
  already does.
- Only an identical hook was checked to be asked once. A hook block in the user's settings
  with another timeout than 600 s may count as a different hook and be asked twice.
- A Bypass session runs every tool without asking anyone; the dialog says so.
