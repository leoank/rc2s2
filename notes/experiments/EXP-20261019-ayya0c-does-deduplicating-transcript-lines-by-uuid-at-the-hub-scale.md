---
id: EXP-20261019-ayya0c
previous_id: manthan/EXP-0012
type: experiment
schema_version: 1
title: Does deduplicating transcript lines by uuid at the hub scale?
project: PRJ-0001
status: concluded         # planned | active | paused | concluded | abandoned
outcome: positive         # null | positive | negative | inconclusive | infra
influenced_by: [EXP-20261016-rq1402, EXP-20261019-ayya0b]
created: 2026-10-19
author: agent:claude
protocols: []
materials: []
tags: [record, phase-1, hub, storage, benchmark]
review: approved
---

## Question
[[EXP-20261019-ayya0b]] found that a restarted node re-reads every transcript and the hub stores each
line again under a new seq. The owner leans towards the hub ignoring a transcript line it
already has, keyed by the line's `uuid` ([[EXP-20261016-rq1402]]), over the node persisting its tail
positions. Does that hold up at the volume a hub will collect: is the uuid a sound key, and
what does a unique index on it cost in SQLite as the table grows?

## Hypothesis and predictions
A `uuid` is unique within a session, so a unique index on (node, session, uuid) drops a
replay and nothing else. Its cost is one more B-tree insert per line, keyed on random
values, so inserts slow down once the index outgrows SQLite's page cache, but stay far above
the rate at which lines arrive, and the index is small next to the payloads. If false: uuids
repeat inside a session (the key would drop real lines), or the insert rate at size falls
near the arrival rate, or the index is a large part of the database.

## Plan
1. Count, read only and without content, this machine's transcripts as the node keeps them:
   lines per active day, line size, and whether a uuid repeats within a session (main and
   subagent files) or across sessions.
2. Benchmark the hub's `events` table in SQLite, set as the hub opens it, grown to 10
   million synthetic lines: no uuid index, a unique (node, session, uuid) index with the
   uuid as text, the same with a 64 MB page cache, and with the uuid as 16 bytes. Measure
   batched inserts per million rows, single-event transactions at size (how the hub stores
   today), a replay of 100,000 stored lines, and file size.

## Success criteria
- No uuid repeats within one session's files.
- At 10 million rows, single-event inserts with the index run at least 1,000 a second (a
  restarted node replays a few thousand lines at once) and within half the rate without
  the index.
- A replay of stored lines adds no rows.
- The index costs under 5% of the bytes the same lines take at this machine's mean line
  size.

10 million lines is about 35 years of this machine at its mean rate, or a year of ten
nodes at its peak day.

## Runs
- [[RUN-20261019-an0gmm]] transcript volume and uuid uniqueness on this machine (plan 1)
- [[RUN-20261019-ank8b1]] the uuid index in SQLite grown to 10 million rows (plan 2)

## Findings
- A transcript line's `uuid` never repeats within a session's files, main and subagents
  together; 145 of 9,044 repeat across two sessions, lines a resumed or forked session
  copies. The key is (node, session, uuid); the uuid alone would lose them
  ([[RUN-20261019-an0gmm]]).
- At 10 million rows the unique index keeps 72% of the single-event insert rate (10,221
  against 14,221 a second) and 65% of the batched rate, slowing gradually with size; a
  replay of stored lines adds nothing at 15,000 a second; it costs 172 bytes a row, 2.2%
  of a line at the mean size ([[RUN-20261019-ank8b1]]). A larger page cache or a binary
  uuid buys nothing worth having.
- This machine writes 766 kept lines on an average active day and 2,613 on its busiest;
  10 million rows is a year of ten such nodes at their peak.
- Built on that: `Event.line_id` carries the uuid (and `usage:<message id>` for the usage
  event a message adds, which a restart replays as well), and the hub's `events` table has
  a partial unique index on (node, session, `line_id`). The live restart check then stored
  no line twice ([[RUN-20261019-aver79]], under [[EXP-20261019-ayya0b]]).
- Proposed for the conclusion: positive; deduplication by uuid at the hub scales well past
  the expected volume.

## Conclusion
Dedup by (node, session, uuid) at the hub scales: 72% of the insert rate and 2% more bytes
at 10 million rows. Adopted.
