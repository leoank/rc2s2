# ADR-0016: The phone embeds tsnet as well

Status: accepted
Date: 2026-10-17

## Context
The Tailscale app on the phone may be joined to another tailnet than the hub's, and it
cannot hold two.

## Decision
The client core embeds libtailscale, cross-built with the NDK; tailscale-rs is the fallback
if it gains Android support first. Until phase 3 the phone uses the Tailscale app on the
hub's tailnet.

## Consequences
Phase 3 work on the Android build.
