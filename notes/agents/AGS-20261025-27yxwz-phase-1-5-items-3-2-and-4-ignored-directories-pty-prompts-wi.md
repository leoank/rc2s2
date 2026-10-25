---
id: AGS-20261025-27yxwz
type: agent_session
schema_version: 1
title: "Phase 1.5 items 3, 2 and 4: ignored directories, PTY prompts with EXP-20261025-25f4bf, ended notifications"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T02:02+00:00
finished: 2026-10-25T02:14+00:00
created_entities:
  - AGS-20261025-27yxwz
  - RUN-20261025-2aymst
edited_entities:
  - PRJ-0001
  - EXP-20261025-25f4bf
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

Start phase 1.5 after committing its plan: item 3 first (ignored directories, so the
experiments' own sessions stop reaching the owner's live hub and phone), then the experiment
that de-risks item 2 and item 2 itself (prompts typed into the PTY), then item 4 (an
`ended` notification and a switch per kind).

## What happened

- **Item 3, node.** `rc2s2 node serve --ignore-cwd <glob>` (repeatable, the `glob` crate):
  `Sessions` holds the patterns; a hook from a matching `cwd` is answered `{}` before
  anything is recorded or held, and registry records there are skipped, so such a session
  is never listed, tailed or held. Unit tests for both paths. In neusis: an `ignoreCwd`
  option on `homeModules.rc2s2-node`, set to `*/target/live-work-*` for rogue in
  `users/ank/ank.nix`, the mirrored test extended; T0 313/313. Not switched: that is the
  owner's `darwin-rebuild`, and until then the live node still sees the test sessions.
- **Item 2, node and app.** `ToWrapper::Prompt { text }`: `rc2s2 run` types the text and
  Enter; the link sends it to a wrapped session's wrapper and uses the socket only for the
  rest, whose note now says the model sees a peer message. No daemon event for a typed
  prompt (the transcript carries the turn). The app's prompt bar says, under the field,
  which path the session has.
- **The experiment.** [[RUN-20261025-2aymst]] for [[EXP-20261025-25f4bf]], a scripted run with
  `rc2s2 ctl` as the client: idle, busy and bypass-mode prompts. All three hold; the busy
  case showed Claude Code queues a typed prompt and fires UserPromptSubmit at submission, so
  the daemon does not need to wait for Stop. Executed twice, the first reading replies too
  early; the driver fixed and the record is the second's. Findings appended, `review:
  pending`.
- **Item 4, hub.** `store_session` says when an update ends a session the hub had as live;
  the notifier then sends an `ended` message (`kind: ended`, the directory as body).
  `--no-notify <kind>` on the hub, checked in `send`. Unit tests for the message and the
  once-only rule. The hub on ghost001 runs the old build until neusis is rebuilt.
- **Decisions made without asking:** `ended` pushes only for sessions the hub had seen
  live, so a node's registration after a hub restart does not announce old endings; the
  app's hint wording; the socket path's note wording.
