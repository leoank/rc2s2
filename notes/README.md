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
  - Phase 1 is done when, from the phone:
    - [ ] every node and every live session is listed, with the conversation streaming live
    - [ ] a permission dialog can be allowed or denied, and falls through to the terminal
      when ignored
    - [ ] a question from Claude can be answered
    - [ ] a session under the wrapper can be prompted and interrupted
    - [ ] a notification arrives when a session blocks or finishes
    - [ ] health shows as a chart per node
- [ ] Phase 2: launch and own sessions: spawn from the phone, kill, credential leases and
  grants, resume, session history
- [ ] Phase 3: confinement and many networks: mxc sandbox profiles, embedded tsnet in hub,
  node and phone, PreToolUse policy rules, a terminal-level adapter, Postgres
- [ ] Phase 4: autonomous starts and more clients: micro VM and container backends,
  triggers, web and iOS clients, integrations

## Open questions
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
