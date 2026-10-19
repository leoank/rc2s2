# ADR-0007: Node health is a time series sampled every 30 seconds

Status: accepted
Date: 2026-10-16

## Context
Node health (question 7, the owner's answer in `../architecture-questions.md`): which metrics, and last state or a series.

## Decision
CPU, memory, disk, network, running claude processes, usage and rate-limit state, and the
GPU when present, every 30 seconds, kept as a series in the hub's database until the owner's
separate monitoring stack takes over.

## Consequences
macOS GPU metrics come from IOKit without sudo (EXP-0009); Linux GPU metrics are still
missing.
