---
id: AGS-20261025-0t3kjw
type: agent_session
schema_version: 1
title: "Phase 1: make the Android app usable and check the phase 1 exit criteria from the phone"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T01:44+01:00
finished: 2026-10-25T01:26+00:00
created_entities:
  - AGS-20261025-0t3kjw
  - RUN-20261025-166zqd
edited_entities:
  - PRJ-0001
  - EXP-20261021-a2ty0f
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The owner asked for phase 1 to continue, with the Android app named as "very ugly and
borderline unusable". Every build step of phase 1 was ticked; what remains are the six
"done when, from the phone" checks on [[PRJ-0001]], and they need an app a person can use.
So: rebuild the app's UI over the unchanged client core, check it on the emulator against a
real wrapped session, and leave the phone checks against the hub on ghost001 to the owner.

## What happened

- **Read** `docs/HANDOFF.md`, `docs/architecture.md`, the project page and
  [[EXP-20261021-a2ty0f]] with [[RUN-20261019-mvz0r3]], then the whole app (four Kotlin
  files, 663 lines): a bare Material 3 shell with no app bar, notices as a card that pushed
  content down, the connect form on every launch, no way back to it once connected, the
  watch's end dropping the person to the connect screen, question options in a row that
  overflowed, a "Message if denied" field always showing, a conversation with no visual
  distinction between roles, 96 dp charts without axes.
- **Rebuilt the UI** (`android/app/src/main/java/me/leoank/rc2s2/`, `Screens.kt` split into
  `Connect.kt`, `Home.kt`, `Session.kt`, `Pending.kt`, `Health.kt`, `Ui.kt`, `Theme.kt`):
  Material 3 with the phone's dynamic colours, top app bars with back navigation, a
  snackbar for outcomes, the connect form only until connected (auto-connect at launch with
  a saved token, a token field that hides its value, the hub's error inline, "Change hub or
  token" in the menu), home with "Waiting for you" cards first then nodes with their
  sessions (status, directory, started, a badge for waiting requests, a terminal icon for
  wrapped sessions, pull to refresh), the session screen with the conversation as chat (user
  bubbles, Claude's text, tool calls as a row with an icon, results folded to three lines,
  very long replies folded), its waiting requests at the end of the conversation, a prompt
  box with Send and a stop button (interrupt, or interrupt-then-send when text is typed),
  Kill behind a confirmation dialog, health with 1 h / 6 h / 24 h charts, gridlines, the
  current value and peak, memory in GB, refreshed every minute. A launcher icon and a
  notification icon of their own; a notification tap opens its session.
- **Model** (`HubModel.kt`): the link state (disconnected, connecting, connected,
  reconnecting); reconnect with backoff when the watch ends, resuming an open session's
  stream after the last seq seen; outcomes as a channel of one-off messages.
- **Core**: a prompt sent over the messaging socket came back in the transcript inside
  Claude Code's "Another Claude session sent a message" banner with a paragraph of advice,
  and the app showed all of it as the person's turn; `conversation.rs` now strips the banner
  (unit test added).
- **Checked on the emulator** by hand first, against a scratch hub and node and a haiku
  session under `rc2s2 run`: question answered, deny with a reason from the dialog, prompt,
  interrupt-then-send, allow, kill; the hub going away shows "Reconnecting" and the app
  recovers when it returns; a wrong token shows "unknown client token" inline. Found and
  fixed on the way: the conversation did not follow new items (the bottom check ran before
  the new layout), "jump to latest" landed on the top of a tall item, the command in a
  permission card was white on white, "0 minutes ago". Then the same steps as a scripted
  run, [[RUN-20261025-166zqd]]: a first execution failed at the end (the conversation did
  not follow new items after a session was reopened, a race between the follow flag and
  the app's own scroll, fixed by letting only the person's drag clear it; and the app was no
  longer on screen when the driver reached the menu, with no crash or process death
  recorded by Android and no reproduction by hand); the second execution passed all five.
- **Phone.** The final build is installed on the owner's phone (SM-F936W, Android 16,
  attached over USB, locked throughout, so nothing was checked there). The phase 1 exit
  checks stay unticked: they are the owner's to do on the phone against ghost001. Push was
  not re-tested; its code changed only in the icon and the tap intent that opens the session.
- **Hook block, on request.** Merged `rc2s2 node hooks --timeout 600` (the daemon's
  address, 127.0.0.1:7777) into `~/.claude/settings.json`, a plain file, not managed by
  neusis; its empty PreToolUse and PostToolUse lists were replaced, every other key kept; a
  copy from before is in the session's scratchpad. Checked with a one-turn `claude -p` on
  haiku: UserPromptSubmit and Stop posted to the daemon and got 200. Claude Code skips HTTP
  hooks for SessionStart, noted as an open question on [[PRJ-0001]].
- **Docs.** `docs/architecture.md` describes the conversation model and the app's screens
  under rc2s2-core. No decision record: the choices are conventional UI details.
- **Not touched.** `notes/tasks.md` was modified in the tree by someone else; the owner's
  commit `d60dba0` (notebook paths) landed during the session.
- **Decisions made without asking:** no navigation library (the three-screen stack stays);
  Send always queues a prompt and the stop button is the interrupt, with interrupt-then-send
  when text is typed, in place of the old "Interrupt, then send" button; Deny asks for an
  optional reason in a dialog instead of a field on every card; Kill confirms; the default
  hub address is `http://ghost001:7700`; versionName 0.2.0.
