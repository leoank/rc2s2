# ADR-0015: libtailscale is linked through a hand-written sys crate built by Nix

Status: accepted
Date: 2026-10-17

## Context
Embedding tsnet (EXP-0006). The published `libtailscale-sys` crate runs `go build` and
downloads modules, which the Nix sandbox forbids; tailscale-rs is a preview without TLS
certs or Android.

## Decision
A `buildGo127Module` derivation builds the static archive; a fifteen-line `build.rs` links
it.

## Consequences
About 27 MB per binary. The Android cross-build is a phase 3 spike of its own.
