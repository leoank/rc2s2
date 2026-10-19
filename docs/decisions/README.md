# Decisions

One architecture decision record per file, `ADR-0001-short-title.md`, numbered in order
and never renumbered. A decision that changes an earlier one supersedes it; both stay.

ADR-0001 to ADR-0012 record the owner's answers in `../architecture-questions.md`
(2026-10-16); ADR-0013 to ADR-0018 the choices the phase 0 experiments settled; the rest
were made while building phase 1 (ADR-0024 and ADR-0025 with the owner's choices for
step 5).

| ADR | Decision | Status |
|-----|----------|--------|
| [ADR-0001](ADR-0001-observe-through-hooks-and-transcript.md) | Observe sessions through hooks and the transcript, drive them through a PTY wrapper, and spawn headless sessions in phase 2 | accepted |
| [ADR-0002](ADR-0002-phase-1-intervention.md) | Phase 1 intervention is allow or deny, answering questions, interrupting and prompting | accepted |
| [ADR-0003](ADR-0003-node-daemons-and-one-hub.md) | One daemon per node keeps an outbound link to one hub on the tailnet; clients talk only to the hub | accepted |
| [ADR-0004](ADR-0004-tailnet-and-pre-shared-tokens.md) | Tailnet only with pre-shared tokens, moving to embedded tsnet on many tailnets | accepted |
| [ADR-0005](ADR-0005-grpc-with-separate-node-and-control-services.md) | gRPC and protobuf on the wire, with separate node and control services | accepted |
| [ADR-0006](ADR-0006-sqlite-in-the-hub.md) | SQLite in WAL mode inside the hub keeps the full event stream, with Postgres later | accepted |
| [ADR-0007](ADR-0007-health-as-a-time-series.md) | Node health is a time series sampled every 30 seconds | accepted |
| [ADR-0008](ADR-0008-push-through-ntfy.md) | Push notifications go through a self-hosted ntfy | accepted |
| [ADR-0009](ADR-0009-rust-core-under-compose.md) | A Rust client core through UniFFI under a Compose app | accepted |
| [ADR-0010](ADR-0010-agent-adapter-trait.md) | A thin agent adapter trait with Claude Code as its one implementation | accepted |
| [ADR-0011](ADR-0011-humans-decide-in-phase-1.md) | Only humans decide in phase 1; policy rules come later | accepted |
| [ADR-0012](ADR-0012-nix-modules-in-the-project.md) | The hub and node Nix modules live in the project until they move to neusis | accepted |
| [ADR-0013](ADR-0013-permission-request-hook-with-keystroke-release.md) | The PermissionRequest hook is the permission callback, held and released on a keystroke | accepted |
| [ADR-0014](ADR-0014-prompts-over-the-messaging-socket.md) | Prompts go over Claude Code's messaging socket; interrupts over the PTY | accepted |
| [ADR-0015](ADR-0015-libtailscale-through-a-hand-written-sys-crate.md) | libtailscale is linked through a hand-written sys crate built by Nix | accepted |
| [ADR-0016](ADR-0016-the-phone-embeds-tsnet.md) | The phone embeds tsnet as well | accepted |
| [ADR-0017](ADR-0017-sandbox-through-mxc.md) | Sandboxed sessions run through mxc behind a Containment trait | accepted |
| [ADR-0018](ADR-0018-credentials-as-leased-slots.md) | Agent credentials are grants the hub leases to nodes as slots | accepted |
| [ADR-0019](ADR-0019-event-seq-from-the-clock.md) | A node numbers its events from the time in microseconds | accepted |
| [ADR-0020](ADR-0020-dedup-transcript-lines-by-id.md) | The hub stores a transcript line once per session by its uuid | accepted |
| [ADR-0021](ADR-0021-client-tokens-and-rc2s2-ctl.md) | Clients authenticate with their own token file, and `rc2s2 ctl` stands in for the phone | accepted |
| [ADR-0022](ADR-0022-held-prompts-reported-from-permission-mode.md) | A prompt to a session that bypasses permissions is reported as held, from the session's permission mode | accepted |
| [ADR-0023](ADR-0023-wrapper-socket-and-terminal-replies.md) | Wrappers reach the daemon over a per-user unix socket, and terminal replies are not keystrokes | accepted |
| [ADR-0024](ADR-0024-android-toolchain-through-the-project-flake.md) | The Android toolchain comes from the project flake, in a shell of its own | accepted |
| [ADR-0025](ADR-0025-push-to-a-configurable-ntfy-topic.md) | The hub pushes to a configurable ntfy topic, a local ntfy in development | accepted |

Each record:

```markdown
# ADR-0001: <the decision, as a sentence>

Status: proposed | accepted | superseded by ADR-xxxx
Date: YYYY-MM-DD

## Context
What forced the decision, and the alternatives considered.

## Decision
What was chosen.

## Consequences
What follows, good and bad.
```
