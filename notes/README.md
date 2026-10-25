---
id: PRJ-0001
origin: manthan/PRJ-0002
type: project
schema_version: 1
title: rc2s2 - A remote command and control session server
status: active            # active | paused | concluded | archived
code: .
created: 2026-10-16
author: ank
tags: []
review: pending
---

## Introduction
There are a lot of existing ways to drive agents remotely.
We do not want to reinvent that, what we are looking for is a highly custom and integrated command and control center.
This should involve writing a server that is aware of all the running agentic session. It should be able to intercept and intervene.
The server should maintain records of all the runs and node health metrics.
We also need small wrappers for claude, antigravity cli, pi and other agent harness to implement the server features.
There should be multiple clients like: android, iOS, web and custom integrations into another app.

## Progress
The phases of the project's `docs/architecture.md`; phase 1 in the steps of `docs/HANDOFF.md`
section 4. Each ticked item says what landed and links the pages with the detail.
- [x] Phase 0: spikes. Nine questions answered, throwaway code under `spikes/`, lessons in
  `docs/HANDOFF.md` (2026-10-17; [[EXP-20261016-rq1401]] to [[EXP-20261016-rq1409]])
- [ ] Phase 1: watch and intervene
  - [x] 1. Cargo workspace and `crates/proto`: event, node and control protobuf packages,
    one bidirectional `Link` stream per node (2026-10-17; [[manthan/AGS-20261017-p8g0qe]])
  - [x] 2. Node daemon, observe only: hook endpoint, transcript tailer with usage once per
    message id, session registry, health with the macOS GPU (2026-10-17; [[EXP-20261017-rwda0a]])
  - [x] 3. Hub, record only: tonic and SQLite, node registration, reconnect with a buffer,
    events stored once by seq and by transcript line id (2026-10-19; [[EXP-20261019-ayya0b]],
    [[EXP-20261019-ayya0c]], [[manthan/AGS-20261019-aycgtw]])
  - [x] 4. Intervention: permission hold and release, AskUserQuestion answers, prompts,
    interrupt and kill (2026-10-19)
    - [x] 4.1 Hold and resolve: the node's hold, the hub's `pending` table and control
      service, `rc2s2 ctl`; allow, deny and fall-through checked live in `-p`
      (2026-10-19; [[EXP-20261019-cqpw0d]], [[manthan/AGS-20261019-bsxr8w]])
    - [x] 4.2 Prompts over the session's messaging socket, with a note when a bypass-mode
      session will hold them at its terminal; live check in 4.4 (2026-10-19)
    - [x] 4.3 The `rc2s2 run` wrapper: Claude under a PTY mirrored to the terminal,
      interrupt (Escape, Ctrl-U, follow-up prompt) and kill from a client, a keystroke
      releasing a held request, terminal replies not counted as typing; live check in 4.4
      (2026-10-19)
    - [x] 4.4 Live checks in an interactive session under the wrapper: AskUserQuestion
      answered from a client, keystroke release, socket prompt, interrupt mid-turn, kill
      (2026-10-19; [[EXP-20261019-fgm60e]])
  - [x] 5. Client core (UniFFI) and the Android app; ntfy push. The Android toolchain comes
    from the project flake with the SDK licence accepted; development runs on an emulator
    on the owner's Mac; the hub pushes to a configurable ntfy topic, a local ntfy in
    development (owner's choices, 2026-10-19)
    - [x] 5.1 Hub: a session's event stream (stored, then live), health history, and ntfy
      notifications for held requests and finished turns (2026-10-19)
    - [x] 5.2 `crates/core`: the client in Rust with UniFFI 0.32 bindings (lists, watch,
      session stream as conversation items, health, allow, deny, answers, prompt, interrupt,
      kill), tested against a real hub; Kotlin bindings generate (2026-10-19)
    - [x] 5.3 Android toolchain in the project flake: `nix develop .#android` with SDK 35, NDK
      29, an arm64 emulator (Android 15 boots headless in 32 s), cargo-ndk, Gradle and a
      rust-overlay toolchain; the core cross-builds for arm64 Android (2026-10-19)
    - [x] 5.4 The Compose app over the core (`android/`): connect, nodes and sessions, pending
      requests with allow, deny and answers, the live conversation with prompt, interrupt
      and kill, health charts; smoke-tested on the emulator against a scratch hub and node
      with synthetic data, allow resolving a held request end to end (2026-10-19)
    - [x] 5.5 Push on the phone through UnifiedPush with the ntfy app as distributor
      (ADR-0026): the hub posts to registered endpoints, the app notifies with Allow and
      Deny; delivered and allowed from the notification on the emulator (2026-10-19)
    - [x] 5.6 Live checks on the emulator against a hub and a node on the Mac: question,
      deny, prompt, interrupt and kill from the app in a real wrapped session (2026-10-19;
      [[EXP-20261021-a2ty0f]])
  - [x] 6. Nix modules: hub (NixOS), node (home-manager, launchd and systemd). Authored in
    the owner's neusis repo; the hub and a self-hosted ntfy run on ghost001, reachable over
    the tailnet, with pre-shared tokens as agenix secrets (2026-10-25; [[AGS-20261024-ta4ae5]])
  - [x] 7. The app made usable: Material 3 screens with navigation, auto-connect and
    reconnect, requests as cards with dialogs, the conversation as chat that follows,
    health charts over a chosen span, a launcher icon; the core shows socket prompts as
    typed. Checked on the emulator against a real wrapped session (2026-10-25;
    [[RUN-20261025-166zqd]], [[AGS-20261025-0t3kjw]])
  - [x] 8. The session screen like the Claude Code terminal, loosely: prompts as boxed
    lines, Claude's text as markdown, tool calls as one line with the output folded to a
    line count until tapped, thinking folded; the core parses every transcript shape (slash
    and `!` commands, task notices, meta text, images, errors) so nothing shows raw
    (2026-10-25; [[AGS-20261025-1mm7j6]])
  - Phase 1 is done when, from the phone:
    - [ ] every node and every live session is listed, with the conversation streaming live
    - [ ] a permission dialog can be allowed or denied, and falls through to the terminal
      when ignored
    - [ ] a question from Claude can be answered
    - [ ] a session under the wrapper can be prompted and interrupted
    - [ ] a notification arrives when a session blocks or finishes
    - [ ] health shows as a chart per node
- [ ] Phase 1.5: fit for daily use. From the owner's notes after using the app
  (`notes/tasks.md` of 2026-10-25, planned with their answers in [[AGS-20261025-25exxv]]);
  ADR-0027 to ADR-0030 in `docs/decisions/`
  - [x] 1. The app's reconnect bug: "Hub object has already been destroyed" shown many
    times. While reconnecting, the closed hub object stayed in place until the new one
    connected, so any command or refresh in between called it; the model now drops it first
    and says "reconnecting" instead (2026-10-25; [[AGS-20261025-25exxv]])
  - [x] 2. Prompts through the PTY for wrapped sessions, the socket for the rest, with the
    app saying which the model will see (ADR-0027); de-risked by [[EXP-20261025-25f4bf]]:
    idle, busy and bypass-mode prompts all arrive as the person's own turn, a busy one queued
    (2026-10-25; [[RUN-20261025-2aymst]], [[AGS-20261025-27yxwz]])
  - [x] 3. `rc2s2 node serve --ignore-cwd <glob>`: sessions under `*/target/live-work-*` are
    not tracked, so the run scripts stop reaching the live hub; `ignoreCwd` in the neusis
    node module, set for rogue, tests mirrored (T0 313/313); takes effect when the owner
    switches (2026-10-25; ADR-0028, [[AGS-20261025-27yxwz]])
  - [x] 4. Notifications: an `ended` kind for a session that exited or was killed, and
    `--no-notify <kind>` on the hub so the policy is tuned by configuration once used;
    unit-tested, takes effect when the hub on ghost001 is rebuilt (2026-10-25; ADR-0029,
    [[AGS-20261025-27yxwz]])
  - [x] 5. Usage in the app (ADR-0030): time and tokens under each turn from
    `turn_duration` and the daemon's usage deltas, the session's totals by model in its
    header and a Usage dialog, cost from a price table by model family kept in the core for
    now (the hub's table comes when the list needs cost); checked against `/cost` by
    [[EXP-20261025-25f8mm]]: the counts match the transcript, `/cost` adds interrupted
    requests (2026-10-25; [[RUN-20261025-2g2812]], [[AGS-20261025-27yxwz]])
  - [ ] 6. Rate-limit utilisation: the daemon reads the access token read only, calls the
    usage endpoint once a minute and reports `usage.five_hour_pct` and
    `usage.seven_day_pct` as health (the names the sampler already gives Claude Code's
    cached value, with `usage.age_s` 0 for a live one), charted per node. Built behind
    `rc2s2 node serve --claude-usage` with a `claudeUsage` option in the neusis module,
    off for rogue; the app charts both windows. Step 1 of [[EXP-20261025-25f6vm]] passed on a
    scratch item ([[RUN-20261025-9fytgj]]); steps 2 and 3, on the real credential, need the
    owner present, and the flag goes on after them (2026-10-25; [[AGS-20261025-92ty85]])
  - [x] 7. The prompt bar and the person's own turns designed properly: a pill input with
    Send and Stop inside it, a state line above it (waiting for you, working with what Send
    and Stop do, started outside the wrapper), the person's turns tinted with the accent
    bar, line breaks kept in Claude's text (2026-10-25; [[AGS-20261025-2vf58e]])
  - [x] 8. Delivery ticks instead of "Sent" toasts: a prompt from the phone shows in the
    conversation at once with one tick when the hub took it and two when the transcript
    echoes it, as messaging apps do; the node's note shows under it, not as a snackbar
    (2026-10-25; [[AGS-20261025-35g9vn]])
  - [x] 9. Questions in bypass-permissions sessions reach the phone: [[EXP-20261025-35gght]]
    found PermissionRequest fires and the node holds the question in bypass mode too; the
    missed one fell in a window when the node was not forwarding (hooks not yet installed,
    or the crash loop after the first rogue switch). No change needed; retry on the phone
    and reopen if it recurs (2026-10-25; [[RUN-20261025-36bknd]], [[AGS-20261025-35g9vn]])
  - [x] 10. A session opens at its most recent messages without scrolling through the
    history: the stored part lands in one piece, the list shows the last screenfuls and
    offers the earlier ones on demand (2026-10-25; [[AGS-20261025-35g9vn]])
  - [x] 11. Diffs visible: an edit's old and new text shown under its row by default, with
    removed and added lines coloured, as the terminal shows them (2026-10-25;
    [[AGS-20261025-35g9vn]])
  - [x] 12. Token labels as `/cost` uses them: "in" is uncached input only, cache read and
    write shown separately as "cached", so a turn does not read as 48k in when its context
    was cached; the header shows in and out (the counts themselves match the transcript,
    [[RUN-20261025-2g2812]]) (2026-10-25; [[AGS-20261025-35g9vn]])
  - [x] 13. The hub link on the phone: a dead link was noticed only when the OS gave up on
    the TCP connection, a connect attempt could hang, and a reconnect backed off to 30 s
    even when the app came back to the front. Now the core's channel sends HTTP/2
    keepalive pings (15 s, 10 s timeout) and gives up connecting after 6 s; the app retries
    from 1 s up to 10 s, retries at once on resume, and checks a connected link with a list
    on resume, reconnecting if that fails; the session screen shows the state too. On the
    emulator the banner shows within 4 s of the hub dying and the link is back 2 s after
    the hub returns, the conversation kept (2026-10-25; [[AGS-20261025-8sxm34]])
  - [x] 14. A session cache: opening a session replayed its whole event stream through the
    hub and the core each time (the usage events too, which is where the wait went); now
    the app keeps what it saw of each session in memory and on disk, shows it at once, and
    resumes the stream after the last item, with the usage totals seeded so the next turn's
    line counts only its own tokens; checked on the emulator on reopen and after a force
    stop (2026-10-25; [[AGS-20261025-8sxm34]])
  - [x] 15. A failed send can be retried by tapping it, and the node's link to the hub and
    the hub's server send keepalive pings as the phone's does, so a machine that slept is
    noticed and its buffered events resent within half a minute of waking (the lost echo
    after rogue's screen turned off). Also from the same notes: the earlier-history control
    is a sticky header ("N earlier · show more"), since the edits the owner looked for sat
    behind it, and bold around code renders (2026-10-25; [[AGS-20261025-9rbnqz]])
  - [x] 16. Keep awake: `rc2s2 node serve --keep-awake` holds `caffeinate -i` while a session
    is busy, `node.keep_awake` as health shown on the node's health screen, `keepAwake` in
    the neusis module, off until the owner turns it on; the switch from the app is a phase 2
    node command (ADR-0031; 2026-10-25; [[AGS-20261025-9rbnqz]])
  - [x] 17. The file diffs of a session that writes files through Bash: the terminal shows
    the unified diff Claude Code computes for the files a command changed
    (`toolUseResult.bashEditDiff`; `structuredPatch` for an edit), so the core carries it
    on the result item and the app shows it under the row, red and green with hunk
    headers; without one, a command of several lines shows whole, the first 15 lines then
    "… N more lines" (2026-10-25;
    [[AGS-20261025-apwmfq]])
  - Phase 1.5 is done when the owner has used the app for a week against ghost001 without
    reaching for `rc2s2 ctl` or the terminal for anything it covers
- [ ] Phase 2: launch and own sessions: spawn from the phone, kill, credential leases and
  grants, resume, session history
- [ ] Phase 3: confinement and many networks: mxc sandbox profiles, embedded tsnet in hub,
  node and phone, PreToolUse policy rules, a terminal-level adapter, Postgres
- [ ] Phase 4: autonomous starts and more clients: micro VM and container backends,
  triggers, web and iOS clients, integrations

## Open questions
- Idle sessions were reported as missing from the app until the owner typed in them
  (2026-10-25). By the time it was looked at they were listed; the daemon scans the
  registry every 2 s, so the cause is unknown. Noted only; reopen if it comes back, with
  the exact observation (session start time against the node's, the hub's session row).
- Should a finished turn push only for sessions touched from the phone (prompted,
  interrupted, or under the wrapper)? Decided after a while of use (ADR-0029).
- Claude Code 2.1.302 skips HTTP hooks for SessionStart ("HTTP hooks are not supported for
  SessionStart" in its debug log), so the daemon first hears of a session from its registry
  entry or its first other hook. Does anything in the daemon depend on SessionStart arriving?
- Remaining after phase 0: see the end of `docs/architecture.md`.

## Design
The architecture is `docs/architecture.md`
and its decision records are in `docs/decisions/` beside it. What a reader must know before
touching the code:
- A daemon per machine observes every Claude Code session through hooks, the transcript
  and Claude Code's session registry, and holds one gRPC stream to the hub; clients talk
  only to the hub, through a separate control service.
- Permission dialogs are held in the PermissionRequest hook and released by a client's
  decision, a keystroke at a wrapped terminal, or a timeout; prompts go over Claude Code's
  messaging socket; interrupt and kill need the `rc2s2 run` wrapper.
- The hub's SQLite record stores each event once, by node and seq and by transcript line
  id, so restarts on either side neither lose nor duplicate anything.

## Lessons
- The published `libtailscale-sys` crate runs `go build` and downloads modules, which the
  Nix sandbox forbids; a hand-written sys crate replaces it ([[EXP-20261016-rq1406]]).
- An idle timer to decide when a held dialog goes back to the terminal is not needed: a
  keystroke there is queued until the hook returns, so the wrapper releases on the
  keystroke itself ([[EXP-20261016-rq1401]], [[EXP-20261016-rq1403]]).
- An MCP `--permission-prompt-tool` for headless sessions is not needed: PermissionRequest
  fires in `-p` mode ([[EXP-20261016-rq1405]]).
- Terminal output cannot be parsed for state: the TUI positions text with cursor moves
  ([[EXP-20261016-rq1403]]). Nor can the workspace trust dialog be driven through the PTY; sessions
  start in trusted directories ([[EXP-20261016-rq1401]]).
- Hooks passed with `--settings` never see SessionStart; the daemon's hooks belong in the
  settings file ([[EXP-20261016-rq1401]]).
- A Claude credential must never be copied into a second store: the first refresh revokes
  the other copy, the owner's own login included ([[EXP-20261016-rq1408]]).
- `touch` inside Claude Code's working directory needs no approval, so it cannot test
  permission handling; use a path outside it ([[EXP-20261019-cqpw0d]]).
- The node's hook block went into the owner's `~/.claude/settings.json` on 2026-10-25
  (`rc2s2 node hooks` merged over the existing keys); it is the manual step the node module
  leaves out, and the hub on ghost001 holds no requests without it ([[AGS-20261025-0t3kjw]]).

## Experiments
- [[EXP-20261016-rq1401]] hold a PermissionRequest in a hook and answer it from another process; positive, works
- [[EXP-20261016-rq1402]] transcript JSONL shapes and where usage and rate-limit state live; positive, mapped
- [[EXP-20261016-rq1403]] PTY injection for prompt, interrupt and keyboard-activity detection; positive, works with caveats
- [[EXP-20261016-rq1404]] answering AskUserQuestion from a hook; positive, multi-select and free text covered
- [[EXP-20261016-rq1405]] headless multi-turn input and the permission callback; positive, resumable
- [[EXP-20261016-rq1406]] libtailscale as a static archive under Nix linked into Rust; positive, builds on both platforms
- [[EXP-20261016-rq1407]] mxc seatbelt sandbox around claude under Nix; positive, policy settled
- [[EXP-20261016-rq1408]] leasing a Claude Code credential into a fresh config dir; active, refresh run pending
- [[EXP-20261016-rq1409]] GPU metrics on macOS without a vendor API; positive, IOKit PerformanceStatistics without sudo
- [[EXP-20261017-rwda0a]] does the observe-only node daemon see what a real Claude Code session does; positive within its scope
- [[EXP-20261019-ayya0b]] does the hub record every node event exactly once across hub and node restarts; positive, with line_id dedup
- [[EXP-20261019-ayya0c]] does deduplicating transcript lines by uuid at the hub scale; positive, adopted
- [[EXP-20261019-cqpw0d]] do held permission requests resolve from a client against real Claude Code; active, allow, deny and fall-through work in -p
- [[EXP-20261019-fgm60e]] does intervention work in an interactive session under rc2s2 run; active, question, keystroke release, socket prompt, interrupt and kill all work
- [[EXP-20261021-a2ty0f]] does the phone app intervene in a real Claude Code session; active, question, deny, prompt, interrupt and kill from the app all land
- [[EXP-20261025-25f4bf]] does a prompt typed into the wrapper's PTY arrive as a plain user turn, and does one typed during a running turn queue rather than interrupt; active, yes to both, queued without interrupting
- [[EXP-20261025-25f6vm]] does the usage endpoint answer with the access token read from the keychain, read only, without disturbing the login; planned
- [[EXP-20261025-25f8mm]] do the transcript's turn_duration lines and the summed usage match what Claude Code's /cost reports; active, counts match the transcript, /cost adds interrupted requests
- [[EXP-20261025-35gght]] does the PermissionRequest hook fire for AskUserQuestion in a bypass-permissions session, and can PreToolUse hold and answer it instead; planned
