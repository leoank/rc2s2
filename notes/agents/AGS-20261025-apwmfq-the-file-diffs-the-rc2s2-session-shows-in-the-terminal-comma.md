---
id: AGS-20261025-apwmfq
type: agent_session
schema_version: 1
title: "The file diffs the rc2s2 session shows in the terminal: commands of several lines shown whole under their row"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T09:45+00:00
finished: 2026-10-25T09:45+00:00
created_entities:
  - AGS-20261025-apwmf
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

"I still can't see the file diffs. Check rc2s2 session." The rc2s2 session is the one this
agent runs in.

## What happened

- **Why.** In this session files are written and patched through Bash: heredocs (`cat >
  file <<'EOF'`) and Python scripts with old and new strings. The terminal shows the whole
  command, so the person sees the file there; the app showed a Bash row with the command's
  first line only (`cd … && cat > Theme.kt <<'EOF'`) and the detail was the description.
  There was never an Edit tool call to show a diff for.
- **Fix.** The core keeps a command of several lines whole as the row's detail (up to 16000
  characters; a single-line command keeps the description). The app opens such a row by
  default, shows the first 15 lines as the terminal does and "… N more lines" to open the
  rest. Checked on the emulator against this very session through a scratch hub and node:
  a Python patch's old and new strings read under its row.
- **The real mechanism, on the owner's second look.** The terminal's green and red diff for
  a Bash command is not the command at all: Claude Code 2.1.302 computes a unified diff of
  the files the command changed and stores it in the transcript line's top-level
  `toolUseResult.bashEditDiff` (`files[].filePath`, `hunks[]` with `oldStart`, `oldLines`,
  `newStart`, `newLines` and ` `, `-`, `+` lines; `changedFiles`; `moreFiles`). Edit and
  Write results carry the same shape as `structuredPatch` with `filePath`. The node already
  forwards the whole line, so the core now reads it into a new `diff` field on the result
  item (`=== path` per file, `@@` hunk lines, the lines as given, clipped at 16000
  characters) and the app shows it under the row by default, file headers relative to the
  session's directory, hunk lines muted; the command itself and an edit's own old and new
  text fold behind a tap when a diff is there. The cache stores the field.
- Phase 1.5 item 17 covers both: the command when there is no diff, Claude Code's diff
  when there is.
