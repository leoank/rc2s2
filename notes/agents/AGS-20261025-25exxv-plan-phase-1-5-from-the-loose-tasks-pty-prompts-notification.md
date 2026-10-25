---
id: AGS-20261025-25exxv
type: agent_session
schema_version: 1
title: "Plan phase 1.5 from the loose tasks: PTY prompts, notification policy, ignored test sessions, usage and cost, the prompt bar, the reconnect bug"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T01:58+00:00
finished: 2026-10-25T02:01+00:00
created_entities:
  - AGS-20261025-25exxv
  - EXP-20261025-25f4bf
  - EXP-20261025-25f6vm
  - EXP-20261025-25f8mm
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The owner wrote six loose notes in `notes/tasks.md` while testing the app and asked for the
right project phase, experiments and decision records, with questions where needed.

## What happened

- **Read the code behind each note first.** Prompts: Claude Code wraps a socket message as
  a peer message with a paragraph of caveats, so the model answers a peer. Idle sessions:
  the daemon scans the registry every 2 s and the hub lists what it sends; no cause found
  in the code. Notifications: the hub pushes on every held request and every Stop hook of
  every session. Test sessions: they run on this machine, so the live node sees them through
  the registry and, since the hook block, through every hook (Claude Code merges the user's
  hooks with `--settings`). "Hub object has already been destroyed": during a reconnect the
  model kept the closed hub object until the new connection succeeded, so a command or a
  refresh in between called a destroyed UniFFI object. Usage: the daemon already records
  tokens per message id and Claude Code writes `turn_duration` lines.
- **Two rounds of questions**, answered: prompts go through the PTY when wrapped and the
  socket otherwise; the idle-sessions note is parked (the sessions showed up again) as an
  open question; notifications keep every kind and gain "ended", tuned later by
  configuration; test sessions are kept out by a node-side cwd pattern; usage shows per
  turn, per session, as rate-limit utilisation and as cost; both the prompt bar and the
  person's turns get a design pass; the work is a new phase 1.5 before phase 2.
- **Written:** ADR-0027 (PTY prompts, amends ADR-0014), ADR-0028 (`--ignore-cwd`),
  ADR-0029 (notification kinds, switchable), ADR-0030 (usage, turn time, rate limits, cost);
  three experiments for the assumptions nobody has checked: [[EXP-20261025-25f4bf]] (typed
  prompts idle, busy and in bypass mode), [[EXP-20261025-25f6vm]] (reading the keychain
  token leaves the login alone; the endpoint answers from the daemon), [[EXP-20261025-25f8mm]]
  (turn_duration and summed usage against `/cost`); phase 1.5 on [[PRJ-0001]] with seven
  items and an exit criterion, the same in `docs/architecture.md` with the prompt, notifier
  and node sections amended; the loose tasks replaced by a pointer.
- **Fixed now, as item 1:** `HubModel.establish` drops the hub object before closing it,
  and a command during a reconnect says "reconnecting" instead of calling a destroyed
  object. Built; not run on a device in this session.
- **Decisions made without asking:** the rate-limit utilisation travels as health metrics
  so the app charts it like CPU; `--no-notify <kind>` as the tuning knob; the `ended`
  kind carries exit or kill; the phase's exit criterion is a week of use without the
  terminal for what the app covers.
