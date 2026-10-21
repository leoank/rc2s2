# ADR-0017: Sandboxed sessions run through mxc behind a Containment trait

Status: accepted
Date: 2026-10-17

## Context
Confining sessions rc2s2 launches (EXP-20261016-rq1407). mxc is a pure Rust SDK over bubblewrap,
LXC, micro VMs and seatbelt; it builds offline under Nix and runs `claude -p` in a seatbelt
profile with hooks and transcript working.

## Decision
mxc behind a `Containment` trait; the daemon owns its own policy format, since mxc's types
have no serde; the hook crosses the boundary as a command hook on a unix socket.

## Consequences
Details in "Sandboxed sessions" of the architecture.
