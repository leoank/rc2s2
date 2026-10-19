# ADR-0004: Tailnet only with pre-shared tokens, moving to embedded tsnet on many tailnets

Status: accepted
Date: 2026-10-16

## Context
Network and auth (question 4, the owner's answer in `../architecture-questions.md`): single user on a tailnet, or reachable from the public
internet. The owner wants to reach several tailnets at once, through tsnet or similar.

## Decision
Phase 1 uses the host's tailscaled on one tailnet and a pre-shared token per node and per
client. Every part gets a listener and dialer boundary so embedded tsnet replaces it in
phase 3 without touching the services.

## Consequences
`serve(listener, hub)` on the hub and `link::dial` on the node are those boundaries. Token
files exist until the credential store holds peer tokens (ADR-0021).
