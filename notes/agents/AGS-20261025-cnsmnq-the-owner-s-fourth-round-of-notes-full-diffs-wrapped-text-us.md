---
id: AGS-20261025-cnsmnq
type: agent_session
schema_version: 1
title: "The owner's fourth round of notes: full diffs, wrapped text, usage alerts"
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-10-25T11:32+00:00
finished: 2026-10-25T11:34+00:00
created_entities:
  - AGS-20261025-cnsmnq
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The three open items in `notes/tasks.md` (the owner's, uncommitted): show diffs in full,
an option to wrap diffs and other text on the phone, and an alert when the usage limit is
being used up very fast or is nearly gone early in the week. Asked the owner first: a
full-screen viewer, one app-wide wrap toggle, alerts pushed by the hub through ntfy, and
all three rules (too fast, near the limit, at the limit).

## What happened

- **Full diffs (item 18).** The core keeps an edit's old and new text, a Write's content, a
  multi-line command and a result's `diff` up to 200,000 characters (`EDIT_CHARS`, which
  replaces `COMMAND_CHARS`). Found on the way: the generic detail clip at 4,000 ran after
  the Bash arm's own 16,000, so a long heredoc was cut at 4,000. The app shows a diff or a
  script's first 15 lines (all of them up to 18) with "… N more lines · tap to see all";
  a tap anywhere on the block opens a full-screen dialog titled by the file, one list row
  per line.
- **Wrap (item 19).** `Wrap`/`LocalWrap` in `Ui.kt`, the choice in `HubModel` kept in the
  app's preferences; a `WrapButton` in the session's bar and the viewer's; `CodeBlock`,
  diffs, scripts and tool output wrap or scroll sideways by it. Off by default, as before.
- **Usage alerts (item 20, ADR-0032).** The node now reports `usage.<window>_resets_s` from
  `resets_at` (the `jiff` crate added to the node to parse it), live and cached. The hub's
  `notify::for_usage` applies the rules on each health sample; `usage_alerts` (migration
  0004) makes each one once per window, reset minute and rule; kind `usage`. The app
  keys a usage notification by its `key` and opens the node's health on a tap.
- **Checks.** `cargo test --workspace` passes (new tests: the core's limits, the hub's
  rules and the once-only claim, the node's reset times on the real response shape); the
  APK builds. On the emulator against a scratch hub (7701, ntfy to a local logger) and node
  watching this machine's sessions: the full-screen view of a `notify.rs` diff, wrapping in
  it and in the conversation, the toggle kept across a force-stop. The scratch hub sent
  two alerts from the real cached values, "7-day Fable limit going fast" (58% with 5 days
  to go, out in 1 day 11 h) and "7-day limit going fast" (40%, out in 3 days), each once
  over seven minutes of 10 s samples. "4 days to go" for 4 days 23 h led to rounding days
  to the nearest.
- **Decisions made without asking:** the thresholds (80%, 95%, pace from 20% used, pace
  for the weeks only); the alert key without the node, so one account's alert is not
  repeated by a second node; wrap off by default; a tap on any diff, short ones too,
  opens the viewer; days rounded in alert text.
- Not checked: a usage push delivered to the phone through UnifiedPush (the payload is the
  same path as the other kinds); the viewer on a very large diff.
