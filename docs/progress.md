# Build progress

What has been built, against the phases in `architecture.md`. The status list says where
each phase 1 step stands; the log below it is newest first, times local (Europe/London).
Append a log entry whenever a crate, module or flow lands, keep it to a few lines, and
update the status list in the same change. Findings belong on the notebook pages, not here.

## Phase 1 status

Steps in the order of `HANDOFF.md` section 4.

- [x] 1. Cargo workspace and `crates/proto`: event, node and control protobuf packages
- [x] 2. Node daemon, observe only: hook endpoint, transcript tailer, session registry, health
- [x] 3. Hub, record only: tonic server, SQLite `events`, node registration, reconnect and buffer
- [ ] 4. Intervention: PermissionRequest hold and release, AskUserQuestion answers, prompts, interrupt and kill
  - [x] 4.1 Hold and resolve: the node's hold, the hub's `pending` table and control service, `rc2s2 ctl`
  - [ ] 4.2 Prompts over the session's messaging socket
  - [ ] 4.3 The `rc2s2 run` wrapper: interrupt, kill, the keystroke release
  - [ ] 4.4 Live checks in an interactive session: AskUserQuestion, prompt, interrupt, keystroke release
- [ ] 5. Client core (UniFFI) and the Android app; ntfy push
- [ ] 6. Nix modules: hub (NixOS), node (home-manager, launchd and systemd)

Phase 1 is done when, from the phone:

- [ ] every node and every live session is listed, with the conversation streaming live
- [ ] a permission dialog can be allowed or denied, and falls through to the terminal when ignored
- [ ] a question from Claude can be answered
- [ ] a session under the wrapper can be prompted and interrupted
- [ ] a notification arrives when a session blocks or finishes
- [ ] health shows as a chart per node

## Log

### 2026-10-19

- **12:15** Step 4, part 1 of 4: hold and resolve. The node holds PermissionRequest while
  linked to a hub (`pending`): it reports a `PendingRequest`, then answers the hook with
  the client's allow, deny or AskUserQuestion answers (`updatedInput`, checked as
  EXP-0004 found), or `{}` when `--hold-secs` (120) runs out or the session ends. Held
  requests ride along in the registration. The hub has a `pending` table (migration
  0002), a registry of linked nodes that forwards commands and waits for their results
  (`live`), and the control service: nodes, sessions, pending, `Watch`, `Resolve`,
  `Prompt`, `Interrupt` (session stream and health come with the client core). Clients
  have their own token file (`--client-tokens`). `rc2s2 ctl` is a command-line client.
  `tests/intervene.rs` covers allow, answers, fall-through, a hub restart while held and
  refused commands. Live with three `claude -p` turns, allow ran the tool, deny stopped it
  with its message reaching the model, and an ignored request fell through after the hold
  ([[EXP-0013]], [[RUN-20261019-bja5]]). Next: prompts over the messaging socket, then
  the `rc2s2 run` wrapper.

- **10:55** Step 3 done. `Event` gained `line_id`: a transcript line's `uuid`, or
  `usage:<message id>` for the usage event a message adds. The hub's `events` table keeps
  one row per node, session and `line_id` (a partial unique index), so a restarted node
  re-reading its transcripts adds nothing. Chosen over persisting the node's tail
  positions after [[EXP-0012]]: the uuid is unique within a session but not across
  sessions (forks copy lines), and at 10 million rows the index keeps 72% of the
  single-event insert rate for 2% more bytes. The live restart check then stored no line
  twice ([[RUN-20261019-htwt]]).

- **07:30** Phase 1 step 3, built; not ticked while the node-restart replay below is open.
  `crates/hub` builds `rc2s2-hub` (`--listen`, `--db`, `--node-tokens`): the `Link`
  service over SQLite (sqlx, migration `0001_record.sql` with `nodes`, `sessions`,
  `events` keyed by node and seq, and `health`), nodes identified by a pre-shared token
  file, registration answering with the last stored seq, every stored event acked.
  `serve(listener, hub)` is the transport boundary for tsnet. The node gained
  `--hub`, `--token-file`, `--name` and `--buffer-events`: `link` keeps unacked events,
  resends them after reconnect and backs off from 1 to 30 s; seq starts at the time in
  microseconds so it rises across restarts. `crates/node` is now a library plus the
  binary. Both binaries colour their logs only on a terminal. `tests/record.rs` covers a
  hub restart, a node restart and an unknown token; the workspace check passes on
  aarch64-darwin and x86_64-linux. Live on this machine every marked hook was stored once
  across a hub outage and a node restart, but a node restart re-reads the transcripts and
  stores each line again under a new seq. Recorded as [[EXP-0011]] with
  [[RUN-20261019-hz3m]] and [[RUN-20261019-xj6y]].

### 2026-10-17

- **22:05** Step 2 verified live: one `claude -p` turn (haiku, Claude Code 2.1.302) in a
  scratch directory with `--settings` from `rc2s2 node hooks` pointed at the daemon.
  UserPromptSubmit, PreToolUse and PostToolUse for Bash, Stop and SessionEnd arrived in
  order under one `session_id`; the transcript path came from the hook input and all six
  kept lines carry the same `sessionId`; three assistant lines with two message ids gave a
  usage total of two messages and 151 output tokens, equal to a sum once per id taken
  straight from the file. A `-p` session also appears in the registry, so pid and busy
  status arrived, and leaving it ended the session. SessionStart is not exercised by
  `--settings`; PermissionRequest, Notification and SubagentStop were not triggered.
  Recorded as [[EXP-0010]] with [[RUN-20261017-qsev]] (live state, read only) and
  [[RUN-20261017-z5ey]] (live hooks).

- **21:45** Phase 1 step 2, built. `crates/node`
  builds the `rc2s2` binary: `rc2s2 node serve` runs the daemon, `rc2s2 node hooks` prints
  the `hooks` block for `~/.claude/settings.json`. Modules: `hooks` (HTTP endpoint on
  127.0.0.1:7777, records each hook and answers `{}`), `transcript` (polls the transcript
  and `subagents/`, keeps `uuid` lines, sums usage once per `message.id` and emits a
  `usage` daemon event with per-model totals; its read state survives a stop so a resumed
  session does not replay), `registry` (`sessions/<pid>.json`, live pids only),
  `sessions` (merges hooks and registry, ends a session on SessionEnd or when it leaves the
  registry, reopens it on a later hook), `health` (sysinfo, IOKit GPU on macOS, the cached
  usage utilization and its age), `outbox` (numbers events under one lock so they leave in
  seq order) and `sink` (JSON lines on stdout until the hub link). `Session` gained a
  `status` field (idle or busy, from the registry). 11 unit tests; the workspace check
  passes on aarch64-darwin and x86_64-linux. A five-second run on the owner's machine found
  the four live sessions with pid and status, backfilled their transcripts, recorded a
  posted hook and sampled health including the GPU. No GPU metrics on Linux yet.

- **21:21** Phase 1 step 1. `Cargo.toml` is a workspace with one member,
  `crates/proto` (`rc2s2-proto`, tonic 0.14, prost 0.14, `tonic-prost-build`). Packages
  `rc2s2.event.v1` (sessions, events carrying the harness's JSON, pending requests,
  resolutions, health samples), `rc2s2.node.v1` (one bidirectional `Link` stream:
  register, events acked by per-node seq, commands back) and `rc2s2.control.v1` (lists,
  `Watch`, `StreamSession`, health, resolve, prompt, interrupt); tokens go in the
  `authorization` metadata. `Connect` was renamed `Link` because it collides with tonic's
  generated client constructor. `tests/node_stream.rs` runs the stream over loopback. The
  flake's dev shell has the Rust toolchain and protoc from nixpkgs; `checks.workspace`
  builds and tests the workspace. Session [[AGS-20261017-imwa]].

- **19:20** Phase 0 closed: nine experiments, [[EXP-0001]] to [[EXP-0009]], all but
  [[EXP-0008]] concluded; throwaway code under `spikes/`; the state and the lessons are in
  `HANDOFF.md`.
