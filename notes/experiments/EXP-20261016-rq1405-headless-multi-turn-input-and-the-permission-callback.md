---
id: EXP-20261016-rq1405
previous_id: manthan/EXP-0005
type: experiment
schema_version: 1
title: Headless multi-turn input and the permission callback
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [intervention, phase-0, headless, prompts, messaging-socket]
review: approved
---

## Question
For spawned sessions: does `--input-format stream-json` keep one process alive across
several user turns, how does the permission callback surface headless, and can a session
started in a terminal be resumed headless? Phase 0 spike 5.

## Hypothesis and predictions
One `claude -p` process accepts many `user` messages over stdin and emits a `result`
per turn. Permission prompts headless go to the tool named by
`--permission-prompt-tool`, an MCP tool we provide, and `--permission-prompts` picks the
target. `--resume <id>` works on any session id regardless of how it started.
`claude --bg` may be a simpler way to own a long-lived session.

## Plan
1. Spawn `claude -p --input-format stream-json --output-format stream-json --verbose`,
   send two user messages ten seconds apart, record every event.
2. A minimal stdio MCP server exposing a permission tool; pass it with
   `--permission-prompt-tool`; trigger a Bash call and record the request and reply.
3. Resume an interactive session's id with `-p --resume` and a follow-up prompt.
4. Try `claude --bg` plus `attach` and `logs` and note what it offers.

## Success criteria
- Two results from one process, with the second turn seeing the first.
- The permission request reaches our tool with the same shape as the hook input.
- Resume across modes works, or its limits are recorded.

## Findings
- **Multi-turn headless works**: one `claude -p` process with `--input-format stream-json`
  accepts many `{"type":"user","message":{"role":"user","content":...}}` lines, answers each
  with a `result`, keeps context, idles between turns, and exits when stdin closes. This is the
  spawner's session model.
- **The permission callback is the PermissionRequest hook** ([[EXP-20261016-rq1401]] run 5): it fires in
  `-p` mode and an allow from the hook runs the tool. No `--permission-prompt-tool` MCP server
  is needed. Hook timeout should be large; a cancel denies the tool in print mode.
- **Resume works in both directions** by session id, so a session started at a terminal can be
  continued headless from the phone and the other way round.
- **`claude --bg` is an alternative spawner**: Claude Code keeps the process, exposes `attach`,
  `logs`, `stop`, `rm`, and `--resume` afterwards. It gives a terminal-attachable session for
  free, but its output is a terminal stream, not events; the daemon would still rely on hooks
  and the transcript. Worth using for phase 2 spawn-from-phone when the person may later
  `attach` at a terminal.
- **Session discovery** without hooks: `~/.claude/sessions/*.json` lists interactive sessions
  with pid, cwd, idle or busy status and a messaging socket path. The socket carries a key and
  a peer protocol used by Claude Code's own session-to-session messaging.
- **The messaging socket is a prompt path** ([[RUN-20261017-dp90a6]], protocol write-up in
  `spikes/10-messaging-socket/PROTOCOL.md`): newline-delimited JSON over the `AF_UNIX`
  socket named in the registry. A daemon of the same user connects and writes
  `{"type":"user","message":{"role":"user","content":"..."}}` (optionally preceded by
  `{"type":"auth","token":...}` from `~/.claude/sessions/<pid>.<hash>.key`; auth was not
  enforced here) and the session takes the text as a user turn wrapped in a
  `<cross-session-message>` banner, acting on it without any terminal input. Delivery is
  gated by the *receiver's* permission mode: a prompting-mode session accepts, a
  bypass-permissions session holds the message for approval in the terminal (an OS
  notification fires) unless `crossSessionInbound: accept` is set. Messages queue and drain
  at the next tool boundary; `priority: "now"` jumps the queue but nothing interrupts, and
  there is no cancel type. Receipts (`control/peer_message_status` with `held | denied |
  expired | delivered | refused | dropped`) and idle notices go only to a reply socket owned
  by the sending pid. So prompts can go over the socket; interrupts stay on the PTY.
- Every turn re-emits `system/init` and a `rate_limit_event` in the stream, so spawned sessions
  give fresh rate-limit state for free ([[EXP-20261016-rq1402]]).

## Runs
- [[RUN-20261017-dp90a6]] messaging socket: protocol, auth, delivery in default and bypass mode
- [[RUN-20261016-rq14ag]] two turns over stdin
- [[RUN-20261016-rq14gq]] headless resume of a terminal session
- [[RUN-20261016-rq14x8]] background sessions and the registry

## Conclusion
- For spawned sessions `--input-format stream-json` keep one process alive across
several user turns
- permission callback surface headless through the PermissionRequest hook
- session started in a terminal can be resumed headless
