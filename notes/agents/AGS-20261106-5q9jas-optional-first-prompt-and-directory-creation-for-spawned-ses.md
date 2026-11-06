---
id: AGS-20261106-5q9jas
type: agent_session
schema_version: 1
title: Optional first prompt and directory creation for spawned sessions, from the owner's first use
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-11-06T05:12+00:00
finished: 2026-11-06T05:12+00:00
created_entities: []
edited_entities: [PHS-20261106-3702ke]
outcome: done             # done | failed | aborted | pending
created: 2026-11-06
author: agent:claude
---

## Task
With phase 2 step 1 on rogue and ghost001, the owner could not start a session in a new
directory from the app, and asked why the dialog needs the prompt up front.

## What happened
- The prompt was required only by the first design. Checked on Claude Code 2.1.318 that
  `claude -p` with stream-json input and no first line registers `idle`, writes no
  transcript, and answers a prompt sent later. Not recorded as a run: one probe by hand.
  In that probe the process did not exit within 5 s of its input closing, but the probe's
  shell held the FIFO, so it says nothing about the spawner, which has its grace kill.
- ADR-0035: the prompt optional; `create` makes a missing directory once its nearest
  existing parent is under a root and the rest is plain names. Decided without asking:
  the app's switch is on by default, since the owner expects new directories to work.
- Unit tests for create (refused without it, outside a root and through `..`) and the
  idle start; checked live against a scratch hub; APK installed on the owner's phone.
- Needs rogue and ghost001 rebuilt to take effect: the node does the creating, and the
  hub carries the new field.
