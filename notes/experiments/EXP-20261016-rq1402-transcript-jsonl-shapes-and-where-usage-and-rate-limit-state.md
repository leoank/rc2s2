---
id: EXP-20261016-rq1402
previous_id: manthan/EXP-0002
type: experiment
schema_version: 1
title: Transcript JSONL shapes and where usage and rate-limit state live
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [observation, phase-0, transcript, usage]
review: approved
---

## Question
What are the line shapes in Claude Code's transcript JSONL, and where can a node read
account usage and rate-limit state? Phase 0 spike 2 plus the first open question.

## Hypothesis and predictions
The transcript has one line per message with `type` in {user, assistant, system,
summary, ...}, assistant lines carrying `message.usage` with token counts, and tool use
and results embedded as content blocks. Rate-limit state is probably not in the
transcript; if so it is in `stream-json` output, a `Notification` hook, or a local
cache file under `~/.claude`.

## Plan
1. Parse every transcript under `~/.claude/projects` and tabulate `type` values and
   the key set per type, with one redacted example line each.
2. Grep transcripts and `~/.claude/*.json` for usage, cost and rate-limit fields.
3. Run one short `-p --output-format stream-json --verbose` session and list the event
   types it emits, looking for usage and rate-limit events.
4. Check whether `Notification` hook payloads carry rate-limit messages.

## Success criteria
- A table of line types and their keys, saved in the sidecar directory.
- A definite source for per-session usage and cost.
- A definite source, or a definite absence, for account rate-limit state.

## Findings

Structure-only survey of every transcript under `~/.claude/projects` (15 project dirs,
27 jsonl files, 12,900 lines, Claude Code 2.1.156 to 2.1.289), full redacted table in
`experiments/EXP-20261016-rq1402/shapes.md`, script in
`code/rc2s2-a-remote-command-and-control-session-server/spikes/02-transcript/shapes.py`.
Values were only inspected in the throwaway sessions of [[EXP-20261016-rq1401]].

### Line shapes

Location: `~/.claude/code/<cwd with every non-alphanumeric replaced by "-">/<sessionId>.jsonl`;
subagents in `<sessionId>/subagents/agent-<agentId>.jsonl` with a sibling
`agent-<agentId>.meta.json` `{agentType, description, toolUseId, spawnDepth, requestShape,
requestNonInteractive}`; large tool outputs in `<sessionId>/tool-results/`. Two families
of line:

| family | `type` | notes |
|---|---|---|
| message lines (have `uuid`, `parentUuid`, `timestamp`, `isSidechain`, `cwd`, `sessionId`, `version`, `gitBranch`, `entrypoint`, `userType`) | `user` | `message.{role, content}`; content is a string for typed prompts or `[{type: text|tool_result}]`; tool results add `toolUseResult`, `sourceToolAssistantUUID`; prompts add `promptId`, `permissionMode`, `promptSource`, `turnOrigin`, `turnPosition` |
| | `assistant` | `message.{model, id, role, content: [{type: text|tool_use|thinking}], stop_reason, usage}` plus `requestId`, `apiBlockIndex`, `effort`, `thinkingDurationMs`, `wireToolInputs`; one line per content block, all sharing `message.id` and the same `usage` |
| | `system` | subtypes `turn_duration` (`durationMs`, `messageCount`), `away_summary`, `informational`, `local_command`, `compact_boundary` (`compactMetadata.{preTokens, postTokens, cumulativeDroppedTokens}`), `stop_hook_summary`, `model_refusal_fallback`; carry `level`, `content` |
| | `attachment` | 31 subtypes in `attachment.type`: `environment`, `model`, `instructions`, `session_context`, `prompt_snapshot`, `deferred_tools_record`, `total_tokens_reminder`, `hook_success`, `hook_permission_decision`, `queued_command`, `edited_text_file`, `file`, ... with an optional `rendered[].content` |
| session-state lines (no `uuid`, keyed by `sessionId`) | `queue-operation` | `operation` enqueue/dequeue/remove, `content` |
| | `last-prompt`, `ai-title`, `mode`, `permission-mode`, `atis-latch` | one scalar each (`lastPrompt`+`leafUuid`, `aiTitle`, `mode`, `permissionMode`, `atis`) |
| | `file-history-snapshot`, `file-history-delta` | `messageId`, `snapshot.trackedFileBackups{<path>: {backupFileName, version, backupTime}}` |
| | `cost-state` | `{totalCostUSD, totalAPIDuration, totalAPIDurationWithoutRetries, totalToolDuration, totalDuration, totalLinesAdded, totalLinesRemoved, startTime, modelUsage{<model>: {inputTokens, outputTokens, thinkingTokens, cacheReadInputTokens, cacheCreationInputTokens, webSearchRequests, costUSD}}, hasUnknownModelCost}`; written once at exit (open sessions have none, twice-resumed ones have two) |

`message.usage` on assistant lines: `input_tokens`, `cache_creation_input_tokens`,
`cache_read_input_tokens`, `output_tokens`, `output_tokens_details.thinking_tokens`,
`server_tool_use.{web_search_requests, web_fetch_requests}`, `cache_creation.{ephemeral_1h_input_tokens,
ephemeral_5m_input_tokens}`, `service_tier`, `speed`, `inference_geo`, `iterations[]`.
The `parentUuid` chain is intact in all 27 files (0 dangling); main transcripts have
`isSidechain: false` throughout, subagent files `true` throughout with `agentId` and the
parent `sessionId`. Stop reasons seen: `end_turn`, `tool_use`, `stop_sequence`, `refusal`.
Nothing in any transcript line names a rate limit, quota or utilization.

### Usage and cost

- Summing `message.usage` grouped by `message.model` after de-duplicating on `message.id`
  reproduces `cost-state.modelUsage` exactly for the model of the main thread (work5:
  3 lines, 2 messages, 4 / 280 / 35,321 / 20,779 in / out / cache read / cache create,
  0.1789 USD). Without the de-duplication every multi-block message is counted twice.
- Over 17 finished sessions output tokens agree at 0.98 to 1.00; cache-read tokens at
  0.22 to 1.00, because `cost-state` also counts calls that leave no assistant line
  (haiku title and classifier calls, warm-ups, retries). Subagent usage is included in
  the parent's `cost-state`. So the transcript sum is a slightly low live estimate and
  `cost-state` is the authoritative figure, available only at exit.
- There is no price table on disk (`cache/model-catalog` has no cost fields,
  `additionalModelCostsCache` is `{}`), so live dollars need a daemon-side table.
- `~/.claude.json` `projects[cwd].{lastCost, lastModelUsage, lastTotal*Tokens,
  lastSessionId, lastGracefulShutdown}` repeats the last interactive session's figures
  per cwd, written at exit, and exists only for some cwds (not the spike dirs).
  `stats-cache.json` has the same `modelUsage` shape but was last computed 2026-09-03
  and is not maintained by 2.1.289.

### Rate-limit state

- Interactive sessions write nothing about rate limits to the transcript, hooks or any
  per-session file. The only on-disk state is `~/.claude.json` `cachedUsageUtilization
  {fetchedAtMs, accountUuid, utilization{five_hour{utilization (percent), resets_at,
  limit_dollars, used_dollars, remaining_dollars, locked_reason}, seven_day{...},
  extra_usage{is_enabled, monthly_limit, used_credits, utilization, spend_limit_reached,
  disabled_reason, ...}, limits[{kind, group, percent, severity, resets_at, scope,
  is_active}], spend{...}, weekly_scoped_shares[]}}`, plus static tier fields in
  `oauthAccount.{userRateLimitTier, organizationRateLimitTier, seatTier,
  hasExtraUsageEnabled}`. It refreshed once in eleven days, at the start of one
  interactive session; three sessions started in the next six minutes did not refresh
  it while the headless runs in the same window reported utilization moving 0.17 to
  0.23. Treat it as advisory with `fetchedAtMs` as its age.
- **What writes it** ([[RUN-20261017-dqeg0a]], 2.1.302 code plus a timed session): only the
  usage loader behind `/usage` (aliases `/cost`, `/stats`, also `claude -p "/usage"`, which
  makes no model call and takes about 4 s) and the SDK control request
  `{"subtype": "get_usage", "skip_behaviors": true}`. No timer, no startup fetch, no write
  from a normal turn or a headless `-p` turn; the refresh seen in the earlier run was a
  `/usage` in that session. A write is skipped when the stored entry is under 60 s old, a
  read ignores entries older than one hour, and `/login` or `/logout` deletes the key, so
  it can be absent. The status line and the stream `rate_limit_event` come from the
  `anthropic-ratelimit-unified-*` response headers, not from this file.
- **The endpoint** ([[RUN-20261017-ebyrjv]]): `GET https://api.anthropic.com/api/oauth/usage`
  with `Authorization: Bearer <access token>` and `anthropic-beta: oauth-2025-04-20`
  answers in about 300 ms with the same `utilization` object the file stores (percent
  values, ISO `resets_at`, `limits[]`, `extra_usage`, `spend`, per-model windows). The
  token needs the `user:profile` scope. Claude Code backs off for `retry-after` or 5 min
  on 403/429.
- Spawned `-p --output-format stream-json --verbose` sessions get a `rate_limit_event`
  per run with `unifiedWindows.{five_hour, seven_day}.{utilization (fraction), resetsAt}`
  and `overageStatus`, straight from the API; this is the only fresh source on the node.
- `policy-limits.json` is org policy (feature restrictions), re-confirmed at session
  start via `policy-limits.json.stamp.json`; not quota. `remote-settings.json` is `{}`.

### Answers for the rc2s2 node daemon

1. Conversation display: tail the `transcript_path` the hooks hand over (or build it from
   `session_id` and cwd as above), keep lines that have a `uuid` and `type` in
   `user | assistant | system`, render `message.content` blocks (`text`, `thinking`,
   `tool_use`, `tool_result`), and ignore `attachment/*` and the uuid-less state lines.
   Order by file position; `parentUuid` gives the tree after a resume or compaction
   (`system/compact_boundary`). `system/turn_duration` marks the end of a turn,
   `queue-operation` and `last-prompt` show queued input, `permission-mode` and `mode`
   track mode changes, `ai-title` gives a display title. Subagents: watch
   `<sessionId>/subagents/`, join by `meta.json.toolUseId` to the parent's `tool_use`
   block. Every message line carries `cwd`, `sessionId`, `version`, `gitBranch` and
   `entrypoint` (`cli` vs `sdk-cli`) so the first line identifies the session.
2. Usage and cost: sum `message.usage` per `message.model` over main and subagent files
   after de-duplicating on `message.id`; price it with a daemon-side table for the live
   figure; replace with the `cost-state` line when it appears at exit. For spawned
   sessions the stream's `result` event (`usage`, `total_cost_usd`, `modelUsage`) is the
   same thing earlier.
3. Rate limits: spawned sessions, from the `rate_limit_event` in their stream, which the
   daemon runs anyway; interactive-only nodes, from `~/.claude.json`
   `cachedUsageUtilization` with its `fetchedAtMs` shown as age (the file is rewritten
   about once a minute while a session runs, so watching its mtime is cheap). Keeping it
   fresh: for accounts the hub holds a grant for, the daemon or hub calls the usage
   endpoint directly at a 60 s or slower cadence; for a person's own login on a node,
   `claude -p "/usage"` on that node rewrites the cache without a model call, or the
   `get_usage` control request when a stream-json session is already open.
   `~/.claude/sessions/<pid>.json` (`status` idle|busy, `cwd`, `sessionId`,
   `/tmp/cc-socks/<pid>.sock`) lists interactive sessions but entries outlive the process.

Recommended daemon approach: one file watcher over `~/.claude/projects` (transcripts plus
`subagents/`), a usage accumulator keyed by `message.id`, `cost-state` as the settle
value, stream-json `rate_limit_event` as the rate-limit feed, the usage endpoint for leased
accounts, `~/.claude.json` as the fallback with an age (absent means unknown), and nothing
from `stats-cache.json` or `policy-limits.json`.

## Runs
- [[RUN-20261016-r0r0j9]] shapes survey, cost-state cross-check, `~/.claude` state files
- [[RUN-20261017-dqeg0a]] what refreshes `cachedUsageUtilization`: code read and a timed session
- [[RUN-20261017-ebyrjv]] the usage endpoint called directly with a leased token

## Conclusion
- We found the shape of the transcripts.
- It is possible to extract usage metrics from it
- One file watcher per daemon is recommended
