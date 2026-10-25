---
id: AGS-20261025-92ty85
type: agent_session
schema_version: 1
title: "Phase 1.5 item 6 without the owner: the daemon reads the credential and reports rate-limit utilisation as health, behind a flag, checked against a scratch item and a fake endpoint"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T08:16+00:00
finished: 2026-10-25T08:45+00:00
created_entities:
  - AGS-20261025-92ty85
  - RUN-20261025-9fytgj
edited_entities:
  - PRJ-0001
  - EXP-20261025-25f6vm
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

Finish what is left of phase 1.5 without the owner: item 6 as far as it goes before the
real credential is touched, and the three items (8, 10, 11) that were done but never
ticked.

## What happened

- **Item 6, node.** A `usage` module behind `rc2s2 node serve --claude-usage`: the credential
  store is the keychain item `Claude Code-credentials` (suffixed by the config dir's hash for
  a non-default dir) read through `/usr/bin/security find-generic-password -w` with a
  ten-second timeout, or `.credentials.json` on Linux; the access token is taken from the
  `claudeAiOauth` block and used only as the bearer on `GET api.anthropic.com/api/oauth/usage`
  with the `oauth-2025-04-20` beta header; the two windows go out as a health sample with
  `usage.age_s` 0. Hidden flags override the endpoint and the service name for tests. The
  sampler already reported Claude Code's cached value under `usage.*`, so the live value
  keeps those names rather than ADR-0030's `claude.*`; the app tells them apart by the age.
  Unit tests for the token, the response and the store choice.
- **Item 6, app.** The health screen charts "Claude 5-hour limit" and "Claude 7-day limit",
  with "live from the usage endpoint" or "from /usage, N ago" under the title.
- **[[RUN-20261025-9fytgj]]** for step 1 of [[EXP-20261025-25f6vm]]: a scratch item and a fake
  endpoint, the node in its own session with no terminal; four polls reached the hub, the
  item was unchanged, the token in no log. Two earlier executions never started the node
  (`setsid` is not on macOS); a Python `os.setsid` replaces it.
- **Neusis.** A `claudeUsage` option on the node module (off), its test; not enabled for
  rogue, since steps 2 and 3 of the experiment, on the real item, need the owner.
- **Ticked** items 8, 10 and 11, done in [[AGS-20261025-35g9vn]] and left unticked.
- **Decisions made without asking:** the `usage.*` names over `claude.*`; the `security`
  tool over a keychain crate; the poller off by default and the flag hidden behind neusis's
  option.
