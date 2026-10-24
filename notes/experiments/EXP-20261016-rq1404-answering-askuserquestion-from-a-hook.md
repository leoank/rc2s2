---
id: EXP-20261016-rq1404
previous_id: manthan/EXP-0004
type: experiment
schema_version: 1
title: Answering AskUserQuestion from a hook
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [intervention, phase-0, hooks, questions]
review: approved
---

## Question
Can a hook answer `AskUserQuestion` so the phone can reply without a terminal? Phase 0
spike 4.

## Hypothesis and predictions
`AskUserQuestion` is a tool, so `PreToolUse` fires with the questions in `tool_input`.
If `updatedInput` can carry an `answers` field the tool returns at once with those
answers; otherwise the terminal prompt still appears and the fallback is the PTY from
[[EXP-20261016-rq1403]].

## Plan
1. Hook server from [[EXP-20261016-rq1401]] with a `PreToolUse` matcher on `AskUserQuestion`
   returning `permissionDecision: allow` and `updatedInput` with the original questions
   plus an `answers` map.
2. Run `claude -p` with a prompt that forces a question and then reports the answer.
3. If that fails, try the same through `PermissionRequest` and record what the terminal
   shows.

## Success criteria
- The assistant reports the hook-supplied answer with no terminal interaction, or the
  negative result is recorded with the exact behaviour seen.

## Findings
- **A hook can answer AskUserQuestion.** The tool input is
  `{questions: [{question, header, options: [{label, description}], multiSelect}]}` and the
  answer is an added `answers` map from question text to the chosen option label, passed back
  as `updatedInput` from either `PreToolUse` (allow) or `PermissionRequest` (allow). The tool
  returns at once with those answers and the terminal shows "User answered Claude's questions".
- **Use the PermissionRequest path.** It fires after PreToolUse, can be held while the phone
  decides, and follows the same release rule as permissions ([[EXP-20261016-rq1401]], [[EXP-20261016-rq1403]]).
  PreToolUse is the fallback.
- **Multi-select and free text** ([[RUN-20261017-e1d84z]], [[RUN-20261017-e35g6f]],
  [[RUN-20261017-e4b03e]]): the `answers` map is keyed by the exact question text. For
  `multiSelect: true` the value is an array of chosen labels (a pre-joined string also
  passes); Claude Code joins it with `", "` before the model sees it, CSV-quoting labels that
  contain the separator. A free-text answer is a plain string that need not match any option;
  there is no "Other" sentinel at the hook layer. The validator rejects unknown question
  keys, arrays on single-select questions, and arrays longer than options + 1. Claude
  receives `answers` as strings in `tool_response` plus a text line; an off-menu answer gets
  the wording "The user answered ... Read the answers carefully", a chosen label gets "Your
  questions have been answered". One map answers several questions in one call. The
  terminal shows "User answered Claude's questions" with one line per question.
- `AskUserQuestion` does not exist in `-p` mode, so spawned sessions never block on it.

## Runs
- [[RUN-20261017-e1d84z]] multi-select answered from the hook
- [[RUN-20261017-e35g6f]] free-text answer not among the options
- [[RUN-20261017-e4b03e]] two questions answered in one call
- [[RUN-20261016-rq146r]] headless: tool unavailable
- [[RUN-20261016-rq14nn]] interactive: PreToolUse answers, no dialog
- [[RUN-20261016-rq14x4]] interactive: PermissionRequest answers

## Conclusion
- A hook can answer `AskUserQuestion`.
