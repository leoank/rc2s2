---
id: AGS-20261106-qt8y8v
type: agent_session
schema_version: 1
title: Spawned sessions fail every permission prompt; add a permission mode such as bypass to spawn
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-11-06T21:40+00:00
finished: 2026-11-06T21:47+00:00
created_entities: [EXP-20261106-qt94y4, RUN-20261106-qtaf57]
edited_entities: [PHS-20261106-3702ke]
outcome: done             # done | failed | aborted | pending
created: 2026-11-06
author: agent:claude
---

## Task
The owner, testing the app, found that a session spawned from the phone fails every
permission prompt, and asked for a way to start a session with flags such as
`--dangerously-skip-permissions`.

## What happened
- Rogue's node log showed the spawn and nothing after; the session's transcript showed
  each tool refused at once ("requires approval"). Rogue's `~/.claude/settings.json` has
  no rc2s2 hooks (empty `PreToolUse` and `PostToolUse`, rewritten 12:09 that day), so
  `claude -p` had no PermissionRequest hook and refused. I did not edit the owner's
  settings file.
- [[EXP-20261106-qt94y4]] ([[RUN-20261106-qtaf57]]): a hook given with `--settings` is asked
  once and obeyed, also when the same hook is in another settings source; both bypass flags
  bypass.
- Built ADR-0037: the spawner passes the node's hooks block as `--settings` to every
  session it starts or resumes; `permission_mode` on `Spawn`/`SpawnRequest`, checked
  against claude's list; `ctl spawn --permission-mode`; a Permissions row in the app's New
  session dialog (Ask, Accept edits, Plan, Bypass). Tests: a spawner unit test for the
  arguments and a refused mode, a refused mode end to end through the hub.
- Checked live on a scratch hub and node: a default-mode spawn's Bash request held and
  allowed from `ctl`; a Bypass spawn ran it unasked. APK built, not installed; the dialog
  was not checked on the emulator.
- Decided without asking, in ADR-0037: a mode rather than free-form flags; no node switch
  forbidding bypass; resume keeps the default mode.
