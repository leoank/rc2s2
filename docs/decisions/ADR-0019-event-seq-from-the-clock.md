# ADR-0019: A node numbers its events from the time in microseconds

Status: accepted
Date: 2026-10-19

## Context
The hub stores each event once by (node, seq) and tells a reconnecting node the last seq
it holds. A counter from zero would repeat after a node restart; a counter persisted on the
node is one more piece of state to keep consistent.

## Decision
Each node process starts its counter at the current time in microseconds and counts up.

## Consequences
Seqs rise across restarts unless the clock goes back or a node sends more than a million
events a second. Re-read transcript lines get new seqs (ADR-0020).
