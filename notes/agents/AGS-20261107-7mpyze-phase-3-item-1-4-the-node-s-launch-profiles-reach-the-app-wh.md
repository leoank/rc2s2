---
id: AGS-20261107-7mpyze
type: agent_session
schema_version: 1
title: "Phase 3 item 1.4: the node's launch profiles reach the app, which offers them on New session and shows a session's profile"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-07T06:57+00:00
finished: 2026-11-07T07:03+00:00
created_entities: []
edited_entities: [PHS-20261106-3702q3]
outcome: done             # done | failed | aborted | pending
created: 2026-11-07
author: agent:claude
---

## Task

The owner asked to commit item 1.3 and go on to 1.4 of [[PHS-20261106-3702q3]]: the
node's launch profiles reported to the hub, picked on the app's *New session*, and shown
on the session.

## What happened

- Committed 1.3 as two commits (code and docs, then notebook); not pushed.
- `LaunchProfile {name, description, sandboxed}` in the shared event proto; the node's
  `Register` and the control `Node` carry the list beside the spawn roots, and the hub
  keeps it with the live link. `Session.profile` names the profile a session runs under;
  the node sets it on a spawn and from a profile's registry scan, and the hub stores it
  (migration 0005, checked on a copy of an older database).
- Core: `NodeInfo.profiles`, `SessionInfo.profile`, `spawn(..., profile)`. App: *Runs as*
  chips on *New session* (this node's user, or each profile, with its description), the
  note under them naming the sandbox's token; the profile on the session card, the
  session header ("spawned in <name>") and the History row. `ctl nodes` lists profiles.
- Decided without asking: a host-backend profile is offered and recorded under its name
  too; a session found only through the host registry after a node restart shows no
  profile, since its transcript is the host's and it resumes on the host anyway.
- Checked: workspace tests (a hub end-to-end test spawns under a host profile and sees
  the profiles on the node and the profile on the session), the APK builds. Not checked:
  the dialog on a device.
