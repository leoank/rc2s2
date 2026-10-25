---
id: AGS-20261025-9rbnqz
type: agent_session
schema_version: 1
title: "Phase 1.5 items 15 and 16: diffs the phone does not show, a retry for a prompt that failed, keepalive on the node's hub link, and whether the node should keep the machine awake"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T08:53+00:00
finished: 2026-10-25T09:02+00:00
created_entities:
  - AGS-20261025-9rbnqz
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

The owner's third round of notes: the diffs the terminal shows are still not on the phone;
no way to retry a prompt that failed, and a prompt the node received whose echo never
showed after rogue's screen turned off; and whether the daemon should keep the machine on.

## What happened

- **Diffs.** Reproduced against the owner's own sessions through a scratch hub and node
  (read only): the Write and Edit rows do render with their coloured lines, but in a long
  session they sit behind the "Show earlier" control at the top of the list, which the list
  opens scrolled past. The control is now a sticky header ("N earlier · show more") that
  stays visible while the list scrolls. The same screenshots showed bold wrapping code
  rendered with its backticks; inline markdown now nests.
- **The lost echo.** The node's link to the hub had no keepalive and no connect timeout, as
  the phone's had not; a sleeping Mac leaves it dead until the OS gives up, and the buffered
  transcript lines wait with it. The node's endpoint and the hub's server now send HTTP/2
  pings (15 s, 10 s timeout) with TCP keepalive and a 6 s connect timeout, so the link ends
  and is remade within half a minute of waking, the outbox resent. A failed send in the app
  says "tap to retry" and resends on a tap.
- **Keep awake.** ADR-0031: `--keep-awake` holds `caffeinate -i` while any session is busy
  and drops it when none is; `node.keep_awake` goes out with health and the node's health
  screen says "keeping the machine awake" or "letting it sleep". `keepAwake` in the neusis
  module, off; the switch from the app is a phase 2 node command, since it needs a command
  through the hub to the node.
- **Decisions made without asking:** idle-sleep only (`-i`), not display sleep; the hold
  tied to busy sessions rather than to the daemon's lifetime; the flag off until the owner
  turns it on.
