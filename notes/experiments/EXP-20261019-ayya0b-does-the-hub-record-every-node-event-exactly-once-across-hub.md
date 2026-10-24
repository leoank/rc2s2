---
id: EXP-20261019-ayya0b
previous_id: manthan/EXP-0011
type: experiment
schema_version: 1
title: Does the hub record every node event exactly once across hub and node restarts?
project: PRJ-0001
status: concluded         # planned | active | paused | concluded | abandoned
outcome: positive             # null | positive | negative | inconclusive | infra
influenced_by: [EXP-20261017-rwda0a, EXP-20261019-ayya0c]
created: 2026-10-19
author: agent:claude
protocols: []
materials: []
tags: [record, phase-1, hub, node]
review: approved
---

## Question
Does the phase 1 hub, record only, store every event a node daemon observes exactly once,
with the node's sessions and health, when the hub restarts while the node keeps observing
and when the node itself restarts? This is the check behind step 3 of phase 1 in the
*Progress* checklist on [[PRJ-0001]]: `crates/hub` (tonic `Link` server, SQLite via sqlx) and the
node's link (`--hub`, a buffer of unacked events resent after reconnect). The daemon it
links is the one checked in [[EXP-20261017-rwda0a]].

## Hypothesis and predictions
If the design holds (seq per node starts at the time in microseconds, the hub returns its
last stored seq on registration, the node resends everything unacked above it, and the hub
ignores a seq it already has), then: events emitted while the hub is down arrive after it
comes back, none is stored twice, seqs stay strictly increasing across a node restart, the
sessions the node knows arrive with its registration, and a node with an unknown token is
refused and stores nothing. If not, the store shows a gap (lost events), a duplicate
payload under two seqs (a resend renumbered), or a seq lower than an earlier one.

## Plan
1. The workspace's integration test `crates/hub/tests/record.rs`, which runs a real hub and
   a real node link on loopback: 50 events, hub stopped, 30 more, hub restarted, then a
   fresh node process with 20 more; plus the unknown-token case. Repeated to catch races.
2. The two binaries on this machine: `rc2s2-hub` on a spare port with a scratch database
   and a generated token, `rc2s2 node serve --hub` against the live machine state (read
   only, as in [[EXP-20261017-rwda0a]]). Post marked synthetic hooks before, during and after a hub
   outage, then restart the node, and summarise the database by kind and source without
   printing content.

## Success criteria
- Every marked hook posted, including those posted while the hub was down, is stored
  exactly once.
- No payload is stored under two seqs; seqs per node increase strictly, also across the
  node restart.
- The live sessions the node found are in `sessions`, and `health` has samples.
- The repeated test run has no failure; an unknown token stores nothing.

## Runs
- [[RUN-20261019-73b0sc]]: the record test, 20 repeats (plan 1).
- [[RUN-20261019-72r8bn]]: hub and node end to end on this machine (plan 2).
- [[RUN-20261019-aver79]]: plan 2 again after the hub ignores re-read lines ([[EXP-20261019-ayya0c]]).

## Findings
- Events the node emits while the hub is down are buffered and stored once after it comes
  back; a hub restart and a node restart each leave every marked hook stored exactly once,
  in order, with seqs increasing across the node restart ([[RUN-20261019-72r8bn]]). The test
  passed 20 of 20 times, and an unknown token stored nothing ([[RUN-20261019-73b0sc]]).
- Sessions arrive with the registration, health samples are stored, including the GPU.
- A node restart stores every transcript line and usage event a second time under new
  seqs: the new process re-reads all transcripts from the start. Exactly-once holds for
  what a node process emits, not across node processes, until the tail positions survive
  a restart or the hub ignores a transcript line it already has (each kept line carries a
  `uuid`).
- A node stopped by SIGINT drops its stream without closing it; the hub logs that as a
  warning.
- After [[EXP-20261019-ayya0c]]: events carry a `line_id` and the hub keeps one per node, session and
  `line_id`. The same live run then stored no transcript line or usage event twice across
  the node restart, with every marked hook still once ([[RUN-20261019-aver79]]); the record
  test gained a node restart that re-sends lines, including one a forked session shares.

## Conclusion
The hub records every event exactly once across hub and node restarts, given the
`line_id` dedup of [[EXP-20261019-ayya0c]].
