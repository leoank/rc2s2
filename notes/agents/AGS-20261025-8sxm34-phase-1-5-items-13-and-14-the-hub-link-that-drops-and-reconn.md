---
id: AGS-20261025-8sxm34
type: agent_session
schema_version: 1
title: "Phase 1.5 items 13 and 14: the hub link that drops and reconnects, and a session cache so opening one is instant"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T08:01+00:00
finished: 2026-10-25T08:07+00:00
created_entities:
  - AGS-20261025-8sxm34
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

Two more notes from the owner: the hub keeps reconnecting on the phone, and opening a
session waits a while before its messages show, which they read as the metrics being
recalculated every time. Phase 1.5 items 13 and 14.

## What happened

- **Why the link dropped.** The core's channel had no keepalive and no connect timeout:
  a phone's link dies silently (sleep, a network change, the tailnet relay moving), the
  app learns of it only when the OS gives up on the TCP connection, a reconnect attempt can
  hang, and the backoff reached 30 s even when the app came back to the front.
- **Item 13.** The core's endpoint sends HTTP/2 keepalive pings every 15 s with a 10 s
  timeout, also while idle, with TCP keepalive at 30 s and a 6 s connect timeout, so a
  dead link ends the streams within half a minute. The app's reconnect backs off from 1 s
  to 10 s; on resume a reconnect in progress retries at once, and a connected link is
  checked with a list, starting the reconnect on a connection error.
- **Why opening waited.** Each open streamed the session's whole event history through the
  hub and the core, usage events included, before the list could land at the end; the
  wait grew with the session.
- **Item 14.** A per-session cache in the app: items, usage totals and the last seq, kept in
  memory while the app lives and written as JSON under the app's files directory (the last
  2000 items) when a session is closed or the model cleared. Opening shows the cache at
  once and streams after its last seq; the core's `stream_session` takes the cached totals
  to seed its usage state, so the first turn line after a resume counts only that turn.
- **Checked on the emulator** against a scratch hub, node and session: a session reopened
  shows its items at once (memory) and after `am force-stop` too (disk, the file under
  `files/sessions/`); with the hub killed the session screen's new banner shows
  "Reconnecting" within 4 s, and 2 s after the hub returns the link is back and the
  conversation still there. Not checked: a phone's real sleep or network change; the
  keepalive numbers come from tonic's defaults for mobile links.
- **Decisions made without asking:** the keepalive and backoff numbers; the cache's
  2000-item cap; a cache that is never invalidated (a session's events are append-only at
  the hub, so a resume from the last seq is exact).
