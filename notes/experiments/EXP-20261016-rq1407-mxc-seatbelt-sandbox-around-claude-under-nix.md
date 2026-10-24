---
id: EXP-20261016-rq1407
previous_id: manthan/EXP-0007
type: experiment
schema_version: 1
title: mxc seatbelt sandbox around claude under Nix
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [confinement, phase-0, sandbox, macos, nix]
review: approved
---

## Question
Does `mxc-sdk` build under Nix on macOS, does its seatbelt backend run `claude`, and
can the sandboxed process reach a localhost hook endpoint and write a transcript that is
visible outside? Phase 0 spike 7 plus the third open question.

## Hypothesis and predictions
`mxc-sdk` is a pure Rust crate that drives `sandbox-exec` on macOS, so it builds offline
once vendored. With a policy of read-write working and state directories, read-only
everything else, and loopback network, `claude -p` runs inside, posts hooks to the
daemon, and leaves its transcript in the mounted state directory. Risk: the SDK may
download a runtime at build or first run, or seatbelt may block the keychain the
credential lives in.

## Plan
1. `spikes/07-mxc/` cargo project depending on `mxc-sdk`, built in the project flake's
   dev shell first, then with `nix build`.
2. Run `echo` under a seatbelt policy; then `curl` to a localhost listener; then
   `claude -p "say hi"` with `CLAUDE_CONFIG_DIR` pointing at a mounted state directory.
3. Record what each policy had to allow for claude to work (nix store, node, keychain,
   network to the API) and whether the hook endpoint was reachable.

## Success criteria
- `mxc-sdk` builds offline under Nix on darwin.
- claude completes a turn inside the sandbox, the hook event arrives outside, and the
  transcript is readable outside.
- The minimum policy is written down; the seatbelt backend's gaps are listed.

## Findings
Spike: `code/rc2s2-a-remote-command-and-control-session-server/spikes/07-mxc/`
(cargo project `mxc-spike`, standalone flake). Policies, logs and probes:
`experiments/EXP-20261016-rq1407/`. Runs: [[RUN-20261016-rq14nq]] build, [[RUN-20261016-rq14sa]]
echo and curl, [[RUN-20261016-rq143k]] claude.

**Build.** `mxc-sdk` 1.0.0 is a pure Rust crate; with default features `build.rs` downloads
nothing (the NanVix fetch is behind the `microvm` feature and Linux/Windows targets) and
the seatbelt backend has no runtime: it calls `sandbox_init()` between fork and exec and
runs `/bin/sh -c <command>` in process. Dev shell `cargo build` 22 s with rustc 1.98.1
(nixos-unstable `55bfb4d7`); `nix build` via `rustPlatform.buildRustPackage` +
`cargoLock.lockFile` 2m56s, offline in the Nix sense; the nix-built binary sandboxes.
No apple-sdk or pkg-config needed. The prediction in the hypothesis holds.

**Three tests.** (a) `/bin/echo hi` passes with an empty policy. (b) `curl` to a host
`127.0.0.1` listener passes with `ingress.default: allow` + `hostLoopback: allow`; with
only `egress: allow` it is refused (the generated profile denies `localhost:*`). (c)
`claude -p` completes a turn, writes its transcript into the mounted config dir where the
host reads it, and a `Stop` hook from inside reaches the host listener.

**Minimum policy for claude** (`policy-claude-min.json`, 3 s per turn):

- read-only: `/nix/store` (or the 63 paths of `nix path-info -r` on the claude
  derivation; both work) and the working directory;
- read-write: the config dir (`CLAUDE_CONFIG_DIR`) and a scratch dir;
- network: `egress.default: allow`; add `ingress.default: allow` + `hostLoopback: allow`
  only for a loopback hook endpoint;
- env: `PATH=<claude wrapper dir>:/usr/bin:/bin`, `CLAUDE_CONFIG_DIR`,
  `CLAUDE_CODE_TMPDIR=<scratch dir>`. `HOME` may name a directory that is not granted.
- credential: `.credentials.json` in the config dir (the lease shape); no keychain access,
  no access to `~` at all.

Two claude-side facts cost most of the attempts: claude creates `/tmp/claude-<uid>` and
ignores `TMPDIR` for it, `CLAUDE_CODE_TMPDIR` moves it; and with `CLAUDE_CONFIG_DIR` set
the keychain item is looked up as `Claude Code-credentials-<sha256(configdir)[:8]>`, so the
host's unsuffixed item is invisible. The keychain route does work with
`keychainAccess: true`, `HOME=<real home>` (search list is HOME-relative), `USER=<name>`
(getpwuid fails under the profile; `id -un` prints `501`) and
`CLAUDE_SECURESTORAGE_CONFIG_DIR=""`, but it grants rw on `~/Library/Keychains` and
`/private/var/folders`, which the lease route avoids. `~/.claude/.credentials.json` on this
machine is a stale June copy; the keychain holds the live one.

**Seatbelt backend gaps** (from the backend doc, confirmed where tested):

- Egress is all-or-nothing plus a loopback exception; no host, CIDR or port rules. A
  per-host allowlist is only possible through a loopback egress proxy the daemon runs, and
  its use is cooperative (`HTTP_PROXY` injected, not transparent redirect).
- Proxy-only egress requires `hostLoopback: deny`, which also cuts the sandbox off from a
  loopback hook endpoint. The two cannot coexist over TCP. **AF_UNIX works instead**
  ([[RUN-20261017-dssgwx]]): with `hostLoopback: deny` a command hook
  `/usr/bin/curl --unix-socket <hooks dir>/hook.sock --data-binary @- http://hook/hook`
  delivers Stop and PermissionRequest to a host listener and the held allow is applied
  (PermissionRequest at +2.95 s, Stop 1.36 s after the answer, 6.2 s wall for the turn).
  The rule from `profile_builder.rs`, confirmed by probes: a read-write path allows
  `network-bind`/`network-outbound` on its subpath, a read-only or denied path denies it,
  and under `egress: allow` the unfiltered outbound rule covers AF_UNIX too, so any host
  socket outside a read-only or denied path is reachable; under `egress: deny` only
  read-write subpaths connect. `hostLoopback` is IP-only. Socket paths must stay under
  the 104-byte `sun_path` limit.
- Inbound is all-or-nothing and not scopeable to 127.0.0.1; `localhost` in a rule means
  every address of the host.
- No CPU, memory or disk limits; only `timeout_ms`. No persistence lifecycle (process
  scoped). Shares the host network stack and Mach bootstrap namespace.
- `/nix/store` is not in the baseline read set; the default child environment is
  `PATH`/`TERM`/`HOME` only with `HOME` = cwd; getpwuid does not work unless
  `extraMachLookups: ["com.apple.system.opendirectoryd.libinfo"]` is added.
- The generated profile is not exposed by the Rust SDK (no dump); `SeatbeltConfig` is
  `#[non_exhaustive]` (construct via `Default`); `ContainerRequest` has no serde, so a
  JSON launch profile needs its own mapping; `sandbox_init` is a deprecated Apple API.
- Audit and learning modes exist for Windows only; macOS policy tuning is by hand.

## Runs
- [[RUN-20261017-dssgwx]] unix-socket hook with loopback TCP denied
- [[RUN-20261016-rq14nq]] build: dev shell and nix build
- [[RUN-20261016-rq14sa]] echo and curl under seatbelt
- [[RUN-20261016-rq143k]] claude -p under seatbelt, eleven attempts

## Conclusion
- `mxc-sdk` builds under Nix on macOS
- seatbelt backend can run `claude`, and
- sandboxed process can reach a localhost hook endpoint and write a transcript but there are some nuances
