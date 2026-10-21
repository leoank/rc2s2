# Handoff: rc2s2 after phase 0

Written 2026-10-17 by the Claude session recorded in [[AGS-20261016-qn108q]] and
[[AGS-20261017-dhk0p8]], for whoever builds phase 1. That session will not resume: the
project is leaving the `yajna` lab for its own repository. Read this, then
`docs/architecture.md`, then the notebook pages it links. Nothing here repeats what those
say; this is the map, the state, and the things that were learned the hard way.

## 1. Where everything is

Paths below are as they stand in the lab repo on branch `rc2s2/core` at commit `2f1f257`.
After graduation (section 3) the code directory becomes the new repo's root and the
notebook pages move to `notes/` inside it, so `code/<slug>/docs/architecture.md` becomes
`docs/architecture.md` and `notes/projects/PRJ-0002-<slug>/experiments/` becomes
`notes/experiments/`.

| What | Where |
|---|---|
| The design, single source of truth | `docs/architecture.md` (499 lines, phases at the end) |
| The decisions behind it, with the owner's answers | `docs/architecture-questions.md` |
| Project page, current belief | notebook `PRJ-0002` README (becomes `notes/README.md`, id `PRJ-0001`) |
| Experiments, one per phase 0 question | `EXP-20261016-rq1401` to `EXP-20261016-rq1409`, under `experiments/` |
| Runs, one page per execution | `runs/RUN-2026101[67]-*.md`, 32 pages |
| Run outputs, scripts, policies, probes | `experiments/EXP-000X/<run-name>/` sidecars |
| Agent session records | `AGS-20261016-qn108q` (phase 0 spikes), `AGS-20261017-dhk0p8` (follow-ups, this handoff) |
| Throwaway spike code | `spikes/` (table below) |
| Project flake | `flake.nix`, flake-parts, nixpkgs follows the root; dev shell holds only python3 today |

Spike directories, each the code behind one experiment:

| Dir | Experiment | What it holds |
|---|---|---|
| `spikes/01-hooks` | EXP-20261016-rq1401, EXP-20261016-rq1403, EXP-20261016-rq1404 | `hook_server.py` (HTTP hook sink that holds PermissionRequest), `hook_server_ask.py` (answers AskUserQuestion), `pty_driver.py`, a dozen `settings-*.json` hook configs |
| `spikes/02-transcript` | EXP-20261016-rq1402 | `shapes.py` (JSONL line shapes), `usage_get.py` (calls the usage endpoint with a credential JSON on stdin) |
| `spikes/05-headless` | EXP-20261016-rq1405 | `multiturn.py`, stream-json multi-turn driver |
| `spikes/06-libtailscale` | EXP-20261016-rq1406 | Nix derivation that builds libtailscale as a static c-archive, a hand-written sys crate, a Rust program that joins a tailnet |
| `spikes/07-mxc` | EXP-20261016-rq1407 | `mxc-spike` cargo project, policies (`policy-claude-min.json`, `policy-claude-unix-hook.json`), `unix_hook_server.py` |
| `spikes/09-gpu-macos` | EXP-20261016-rq1409 | IOKit `IOAccelerator` sampler via `io-kit-sys` |
| `spikes/10-messaging-socket` | EXP-20261016-rq1405 | `PROTOCOL.md` (the reverse-engineered socket protocol), `sink.py`, `daemon.py`, `probe.py` |

Spikes 03, 04 and 08 had no code of their own; their scripts sit in the sidecars
(`experiments/EXP-20261016-rq1408/run-refresh/` and `run-twosession/` hold the credential tooling:
`run_lease.sh`, `cred_state.py`, `store.py`, `idle_session.py`).

## 2. State on 2026-10-17

- Phase 0 is done. Nine experiments; eight are `concluded`, `approved`, with conclusions
  written by the owner. `EXP-20261016-rq1408` (credential leasing) is still `review: pending` with
  one planned run, [[RUN-20261017-m3gg0y]], deferred (section 5).
- No product code exists. Not a crate, not a proto file. The repository layout in
  `docs/architecture.md` ("Repository layout") is the target, not the state.
- The architecture doc's "Open questions" section reads "None left from phase 0".
- Every finding from the runs on 2026-10-17 is folded into `docs/architecture.md`; the
  runs are linked from the EXP pages' Findings and Runs sections.

## 3. The migration the owner is doing

Planned by the owner, not yet done when this was written:

1. Merge branch `rc2s2/core` into `main` of the lab. The branch is two commits on top of
   `cc33196`; `main` moved on with the Rust `yajna` CLI (commits `497e8e1` to `83b1851`).
   The branches touch disjoint paths and `git merge-tree` reports a clean merge. The
   Rust CLI validates the branch's 47 pages.
2. Graduate: `yajna graduate PRJ-0002 --to github:leoank/rc2s2 --into <path>` from the
   lab (ADR-0018, `docs/conventions.md` "Graduation" in the lab). It subtree-splits the
   code directory with history, imports the notebook as `notes/`, marks the lab page
   `moved`, drops the flake input, registers the namespace. Left to the owner: the lock,
   the commits in both repos, the remote, the progress log line.
   Pass `--into` explicitly: the default is `../<namespace>` next to the lab, and with the
   lab's worktree layout (`~/workspace/chaos/yajna/{.bare,main,rc2s2/core}`) that resolves
   to the directory that already holds the `rc2s2/core` worktree.
3. Make a worktree in the new repo and continue there.

What to check first in the new repo:

- `yajna validate` at its root. The project page is renumbered `PRJ-0001`; experiment and
  run IDs are unchanged; links into the lab became `[[yajna/...]]`.
- `nix develop` at the root still enters the project shell; the root flake of the lab no
  longer lists the project.
- `docs/architecture.md` links such as `[[EXP-20261016-rq1401]]` are outside the vault and not
  validated; they still name the right pages because IDs survived.
- The lab's copy of the project is frozen. Never edit it.

## 4. What to build next: phase 1

The order that makes each step testable with what exists:

1. **Workspace and proto crate.** Cargo workspace per the layout section; add the Rust
   toolchain and `protoc` to the project flake (rust-overlay or crane as a project
   input, then `nix flake lock` in the project). Two protobuf packages from day one,
   node service and control service, event types shared.
2. **Node daemon, observe only.** HTTP hook endpoint on localhost, transcript tailer,
   session registry from `~/.claude/sessions/*.json`, health sampler. Verify with a hand
   started `claude` on the same machine: every hook event lands, the transcript lines
   match the hook's `session_id`, usage sums once per `message.id`. The settings file is
   the user's `~/.claude/settings.json` (SessionStart only fires from there).
3. **Hub, record only.** tonic server, SQLite via sqlx, the `events` table, node
   registration over one bidirectional stream with reconnect and the node-side buffer.
4. **Intervention.** PermissionRequest hold and release on the daemon with the keystroke
   release rule; the hub's `pending` table; AskUserQuestion answers via `updatedInput`;
   prompts over the messaging socket; interrupt and kill through the PTY wrapper
   (`rc2s2 run`). Each has a tested shape in a spike; port, do not rediscover.
5. **Client core and Android.** UniFFI core, Compose UI over it; ntfy for push.
6. **Nix modules** for the hub (NixOS) and the node (home-manager, launchd and
   systemd); they move to the owner's neusis repo later.

Phase 1 ends when the six bullets under "Phase 1: watch and intervene" in the
architecture doc hold from the phone. Credential leasing, spawn and sandboxing are phase
2 and 3; the design for them is already settled, so build the transport and adapter
boundaries so they slot in (the `Containment` trait, the adapter trait, the listener and
dialer boundary for tsnet).

## 5. Deferred and untested

- [[RUN-20261017-m3gg0y]]: two sessions sharing one credential store across a forced
  refresh. Planned, scripts ready. Needs a fresh `claude auth login` into a scratch
  `CLAUDE_CONFIG_DIR` by a human (browser flow), never a copy of a live login. The
  design already assumes the answer is "fine" on the strength of the code read.
- Credential save path inside a seatbelt sandbox without keychain access: a refresh there
  must land in the file, since the keychain item is unreachable. Untested; noted in
  EXP-20261016-rq1407 and EXP-20261016-rq1408.
- libtailscale cross-build for Android with the NDK: a phase 3 spike of its own.
- Whether the usage endpoint needs the `user:profile` scope on every account kind; it
  worked on the owner's team account.
- `crossSessionInbound: accept` for spawned sessions was read from the code, not
  exercised; the hold-for-approval behaviour in bypass mode was observed.

## 6. Nuances, by subsystem

Everything here was observed on Claude Code 2.1.302 (Nix store binary, bun-compiled; the
wrapper adds `--plugin-dir`). Minified identifiers change per release, so code facts
below are described by behaviour, not symbol.

**Credentials** (EXP-20261016-rq1408, RUN-5jwo, RUN-tnz7)

- A Claude OAuth credential is a grant. A refresh rotates access and refresh token and
  revokes the previous pair immediately. Two stores holding one grant: the first to
  refresh kills the other. This logged the owner out once and killed four subagents.
- Refresh fires when `now + 300 s >= expiresAt`; `POST platform.claude.com/v1/oauth/token`
  with `grant_type: refresh_token`. Access token lives 8 h, refresh token ~30 days from
  login. `invalid_grant` clears the store to empty tokens.
- The store is compare-and-swap under a lock file in the config dir, with adopt-the-sibling
  on conflict; running sessions re-read on mtime or keychain version change. Many sessions
  on one store is the normal case.
- `CLAUDE_CONFIG_DIR` selects the store. On macOS the keychain item is
  `Claude Code-credentials-<sha256(dir)[:8]>` (unsuffixed for the default dir);
  `CLAUDE_SECURESTORAGE_CONFIG_DIR` overrides which dir is hashed. A refreshed pair goes
  to the keychain and `.credentials.json` is deleted, so read-back checks the keychain
  first. On Linux the file is the store.
- `claude auth status` prints `loggedIn` JSON with no network turn. `claude auth logout`
  revokes at the server (a saved token gets 401 afterwards). `claude auth login` is
  browser-only; no headless flow.
- Never copy the owner's live credential anywhere a refresh could happen. Never print a
  token; the sidecar scripts print 8-hex hashes only.

**Hooks and permissions** (EXP-20261016-rq1401, EXP-20261016-rq1404)

- Hooks in `--settings` files miss SessionStart; the daemon's hooks belong in
  `~/.claude/settings.json`.
- PermissionRequest blocks the dialog; keystrokes typed meanwhile are queued and applied
  after the hook returns. Release the hook with an empty response as soon as the PTY
  wrapper sees a keystroke. `Notification` for `permission_prompt` only arrives after the
  hook returns, so PermissionRequest is the push trigger. Hook `timeout` is enforced to
  the second, default 600 s.
- AskUserQuestion: allow with `updatedInput` = original input plus `answers`, keyed by
  the exact question text. Single-select: the label. Multi-select: an array of labels
  (joined with ", " and CSV-quoted; a pre-joined string is accepted too). Free text: any
  string, no "Other" sentinel. Arrays on a single-select question or longer than options
  plus one are rejected. The `tool_result` wording differs for off-menu answers. The tool
  does not exist in `-p` mode.

**Messaging socket** (EXP-20261016-rq1405, RUN-ejq2, `spikes/10-messaging-socket/PROTOCOL.md`)

- `~/.claude/sessions/<pid>.json` names `/tmp/cc-socks/<pid>.sock` (0600 in a 0700 dir)
  and a key file `<pid>.<64hex>.key` with `{peerToken, pidDomain, procStart}`. Entries
  outlive their process; check the pid.
- NDJSON. `{"type":"user","message":{"role":"user","content":"..."}}` becomes a queued
  user turn in a `<cross-session-message>` banner. Optional `session_id`, `priority`
  (`now|next|later`), `from` (`uds:` address), `from_mode`, `file_attachments`.
  `{"type":"auth","token":...}` is accepted but was not required. `control` actions:
  `rename`, `peer_message_status`, `notify_when_idle`, `peer_idle_notice`.
- Peer identity is SO_PEERCRED (same uid). Receipts and idle notices go only to a reply
  socket owned by the sender pid. No interrupt message exists.
- A bypass-permissions session holds inbound messages for approval at its terminal
  (OS notification) unless `crossSessionInbound: accept` is set.

**PTY** (EXP-20261016-rq1403)

- Interrupt: Escape, pause, Ctrl-U (Claude refills the box with the interrupted prompt),
  then text and Enter. No Stop hook fires for an interrupted turn.
- Strip every `CLAUDE*` variable and `AI_AGENT` before spawning; the trust dialog cannot
  be driven; `--dangerously-skip-permissions` shows a one-time warning screen on a PTY.
- Never parse terminal text for state; it is cursor-positioned and unreliable.

**Transcript and usage** (EXP-20261016-rq1402, RUN-trj6, RUN-ho7k)

- Keep `uuid`-bearing `user`, `assistant`, `system` lines; ignore attachment and state
  lines; watch `subagents/` next to the file; de-duplicate on `message.id` before summing
  usage (one API message is one line per content block).
- `cachedUsageUtilization` in `~/.claude.json` is written only by `/usage` (`/cost`,
  `/stats`, `claude -p "/usage"` which takes ~4 s with no model call) and the SDK control
  request `get_usage`; 60 s write throttle, 1 h read TTL; `/login` and `/logout` delete it.
  The status line and `rate_limit_event` come from `anthropic-ratelimit-unified-*` headers.
- `GET https://api.anthropic.com/api/oauth/usage` with `Authorization: Bearer <access>`
  and `anthropic-beta: oauth-2025-04-20` returns the same utilization object in ~300 ms
  (`five_hour`, `seven_day`, `limits[]`, `extra_usage`, `spend`, `weekly_scoped_shares`).
  Shape saved in `experiments/EXP-20261016-rq1402/run-usage-endpoint/`.

**Sandbox** (EXP-20261016-rq1407, RUN-syvm)

- `mxc-sdk` 1.0.0 builds offline under Nix with default features; seatbelt backend has no
  runtime. Request types have no serde and the generated profile cannot be dumped, so the
  daemon owns its own policy format.
- Claude inside needs: read-only `/nix/store` and the working dir; read-write config dir
  and scratch dir with `CLAUDE_CONFIG_DIR` and `CLAUDE_CODE_TMPDIR` (it ignores `TMPDIR`);
  `PATH` of the wrapper plus `/usr/bin:/bin`; outbound network; no home, no keychain.
- The hook crosses the boundary as a command hook:
  `/usr/bin/curl -s --max-time 55 --unix-socket <dir>/hook.sock -X POST -H 'Content-Type: application/json' --data-binary @- http://hook/hook`,
  with `hostLoopback: deny`. Loopback TCP on seatbelt means all inbound.
- `egress: allow` also covers AF_UNIX: the sandbox can connect to any host unix socket
  not under a read-only or denied path, including `/tmp/cc-socks/*.sock`. Deny that path
  explicitly, or use `egress: deny` plus a proxy. `hostLoopback` is IP-only. `sun_path` is
  limited to 104 bytes, so keep socket paths short.

**Networking and GPU** (EXP-20261016-rq1406, EXP-20261016-rq1409)

- libtailscale builds as a static c-archive with `buildGo127Module` and
  `-buildmode=c-archive`; a hand-written sys crate links `static=tailscale` plus
  CoreFoundation and Security on Apple targets. The published `libtailscale-sys` crate
  downloads at build time; do not use it. About 27 MB. Built on aarch64-darwin and, via
  the remote builder, x86_64-linux.
- The phone must embed tsnet too (decided 2026-10-17): its Tailscale app may be on a
  different tailnet than the hub's.
- macOS GPU: IOKit `IOAccelerator` `PerformanceStatistics` through `io-kit-sys`, under
  100 µs per sample.

**Claude Code internals, how they were read**

- `strings` over the Nix store binary, then grep with context. Part of the binary is
  bytecode, part plain minified JS; the JS part carries the credential, usage and socket
  logic. The helper used for this lived in a session scratchpad and was not kept; it was
  a dozen lines.

## 7. Rules the owner set for agents on this project

- Secrets: never print, log or write a token, key or keychain credential value. Do not
  read `~/.claude/.credentials.json`. Never write the unsuffixed `Claude Code-credentials`
  keychain item. Hashed test items are fine and must be deleted afterwards.
- Never touch the owner's live Claude sessions or their credential store beyond reading
  state. Never run `claude stop`, `logs` or `rm` with guessed ids.
- Nix: tools come from the flake, never ad hoc installs; never edit `/nix/store`; never
  pass `--builders ''`; agenix secrets stay encrypted.
- Git: commit only when asked, stage by path, conventional subjects, no trailers; never a
  bare `git stash`, the stack is shared across worktrees; work on the branch you are on.
- Notebook: pages via `yajna new` only; a `RUN` page per run with its sidecar; findings on
  the `EXP` page with `review: pending`; conclusions are written by the owner; an `AGS`
  page per session; `yajna log --author agent:claude`; `yajna validate` before reporting.
- Subagents working on this must not touch the owner's running sessions; they share the
  owner's login and die together with it if it is revoked.

## 8. Resources

- Claude Code docs: https://code.claude.com/docs/en/hooks (hooks reference),
  https://code.claude.com/docs/en/headless (`-p`, stream-json), https://code.claude.com/docs/en/settings,
  https://code.claude.com/docs/en/security.
- Tailscale tsnet: https://tailscale.com/docs/features/tsnet; libtailscale:
  https://github.com/tailscale/libtailscale; Tailscale's Rust tsnet preview is
  `tailscale-rs` on crates.io.
- mxc: crate `mxc-sdk` 1.0.0 on crates.io (Microsoft sandboxing SDK).
- UniFFI: https://mozilla.github.io/uniffi-rs/. ntfy: https://ntfy.sh.
- The owner's fleet config (hub and node modules go there eventually):
  https://github.com/leoank/neusis.
- The lab repo and its conventions: https://github.com/leoank/yajna, `AGENTS.md`,
  `docs/conventions.md`, ADR-0018 for graduation and namespaced links.
