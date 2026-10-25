---
id: AGS-20261025-35g9vn
type: agent_session
schema_version: 1
title: "Phase 1.5 items 8 to 12 from the second round of notes: delivery ticks, questions in bypass mode, open at the end, visible diffs, token labels"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T02:53+00:00
finished: 2026-10-25T03:00+00:00
created_entities:
  - AGS-20261025-35g9vn
  - EXP-20261025-35gght
  - RUN-20261025-36bknd
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The owner's second round of notes from using the app: the "Sent" toasts, a question that
never reached the phone, the conversation scrolling through its whole history on open, no
visible diffs, and token counts that looked too high. Fold them into phase 1.5 and build.

## What happened

- **Items 8 to 12** added to phase 1.5 on [[PRJ-0001]]; the loose tasks replaced by a pointer.
- **Item 12, tokens.** The counts were right (they equal the transcript's,
  [[RUN-20261025-2g2812]]); the label was wrong: "in" summed cached context with uncached
  input, so a turn read "48k in". Now, as `/cost` words it, "in" is uncached input, the
  cached context is its own "cached" figure, and the header shows in and out.
- **Item 9, the missed question.** [[EXP-20261025-35gght]] with [[RUN-20261025-36bknd]]: in a
  bypass-permissions session PermissionRequest fires for AskUserQuestion after PreToolUse
  and the node holds it, so the hold works in the owner's mode too. The missed question fell
  in a window when the node was not forwarding: before the hook block existed, or during
  the crash loop after the first rogue switch. No code change; reopen if it recurs.
- **Item 11, diffs.** The core writes an edit's detail as "- " and "+ " lines (Write as all
  "+ "); the app opens an edit's row by default and colours removed lines red and added
  lines green; the edit's own result line ("1 line") stays folded.
- **Item 10, open at the end.** While a session opens, the model gathers the stored items
  until the stream goes quiet for 250 ms and shows them in one piece, so the list lands at
  the end; the view renders the last 60 rows with a "Show earlier · N more" button that
  adds 100 at a time.
- **Item 8, ticks.** No "Sent" or "Interrupted and sent" snackbars. A prompt from the phone
  appears in the conversation at once, grey with a hollow mark; one tick when the hub took
  it (the node's note, such as the socket path's, under it; a failure in red); when the
  transcript echoes the same text the local turn goes and the real one shows two ticks.
- Checked on the emulator: a Write and an Edit with their diffs open, a prompt sent from
  the app arriving with one then two ticks, the token labels. The "Show earlier" button and
  the quiet-stream landing were not exercised (a session with over 60 rows was not made).
- **Decisions made without asking:** the window of 60 rows and the 100-row step; matching
  the echo by exact text; ticks as "✓" and "✓✓" glyphs rather than icons.
