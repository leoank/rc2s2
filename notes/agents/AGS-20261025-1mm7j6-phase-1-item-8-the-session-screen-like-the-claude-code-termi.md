---
id: AGS-20261025-1mm7j6
type: agent_session
schema_version: 1
title: "Phase 1 item 8: the session screen like the Claude Code terminal, outputs collapsed, every transcript shape parsed"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T01:29+00:00
finished: 2026-10-25T01:43+00:00
created_entities:
  - AGS-20261025-1mm7j6
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The owner added a phase 1 item: the session screen should look like the Claude Code
terminal, loosely, with outputs collapsed by default, and the many transcript shapes the app
still showed as raw markup had to be parsed. Design was left to my judgement.

## What happened

- **Found the raw shapes** by tallying the user, assistant and system lines of the owner's
  forty most recent transcripts through the core's rules (shapes only, no content kept):
  `<task-notification>` blocks, `<command-name>` slash commands and their
  `<local-command-stdout>` (ANSI inside), `<local-command-caveat>`, `<bash-input>` and
  `<bash-stdout>`/`<bash-stderr>` from `!` commands, `isMeta` lines (skill bodies, hook text),
  `[Request interrupted by user]`, `<system-reminder>` blocks appended to prompts, tool
  results with images or tool definitions, system lines of subtype `local_command`,
  `compact_boundary` and `away_summary`, tool calls with no known key input (WebSearch,
  ToolSearch, Skill, SendMessage, TodoWrite, MCP tools), and markdown in Claude's text.
- **Core** (`crates/core/src/conversation.rs`): `Item` gained `tool`, `detail` and `error`;
  `Role::Thinking` folds Claude's thinking. Each shape above becomes a readable item or is
  dropped; tool calls get a short name (MCP prefixes removed), the one input that matters and
  a detail (an edit's diff, a file's content, an agent's prompt, a todo list, else the input
  as JSON); results keep 4000 characters now that the client folds them; the routine
  permission and idle notifications are dropped because the pending card and the session's
  state already say so. Tests cover every shape.
- **App**: `Markdown.kt`, a small renderer (headings, lists, fenced code, tables as mono,
  quotes; inline code, bold, italic, links); `Session.kt` rows: the prompt as a boxed line
  with an accent bar, Claude's text as markdown, a tool call as one line (icon, name, input,
  paths relative to the session's directory) with the output folded to a line count or the
  error's first line until tapped, thinking folded, notices as captions. A call and its
  result pair even with a notice between them.
- **Checked on the emulator** against a scratch hub, node and a haiku session under the
  wrapper: a markdown reply, Bash, Write, Edit (the diff in the detail), a failing command,
  `/context` and `!pwd` typed at the terminal; every one readable, folded, and opening on a
  tap. Fixed on the way: nested code fences broke the parser (a fence now closes on a bare
  ``` only), the prompt's accent bar did not draw, "1 lines", standalone outputs could not
  open, the permission notice split a call from its result.
- **Decisions made without asking:** no markdown library, a renderer of a screen's worth
  of rules instead; thinking shown, folded, clipped to 2000 characters; the two routine
  notifications dropped rather than reordered; `away_summary` system lines dropped.
- The build is installed on the owner's phone (locked, unchecked there). Not a scripted
  run: the visual check is recorded here, with the screenshots left out of git.
