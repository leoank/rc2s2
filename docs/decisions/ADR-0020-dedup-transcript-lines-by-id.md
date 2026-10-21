# ADR-0020: The hub stores a transcript line once per session by its uuid

Status: accepted
Date: 2026-10-19

## Context
A restarted node re-reads every transcript and sends each line again under a new seq
(EXP-20261019-ayya0b). Alternatives: persist the tail positions on the node, or let the hub recognise a
line it already has. EXP-20261019-ayya0c measured the second: uuids are unique within a session but
repeat across forked sessions; at 10 million rows a unique index keeps 72% of the insert rate
for 2% more bytes.

## Decision
Events carry `line_id` (the line's uuid, or `usage:<message id>`), and a partial unique
index on (node, session, `line_id`) drops repeats.

## Consequences
No node state to persist; every restart still re-reads and re-sends the transcripts, which
costs bandwidth, not storage.
