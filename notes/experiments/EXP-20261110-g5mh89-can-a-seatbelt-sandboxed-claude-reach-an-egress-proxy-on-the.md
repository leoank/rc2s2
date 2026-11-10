---
id: EXP-20261110-g5mh89
type: experiment
schema_version: 1
title: Can a seatbelt-sandboxed claude reach an egress proxy on the node that adds a setup-token as its credential, with a dummy token inside and no TLS interception
project: PRJ-0001
status: planned           # planned | active | paused | concluded | abandoned
outcome: null             # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-11-10
author: agent:claude
protocols: []
materials: []
tags: [credentials, phase-2]
review: pending
---


## Question
Under ADR-0039 an egress proxy on the node should hold the token and add it to requests
to the Anthropic API, so a sandboxed session holds only a dummy. On seatbelt the sandbox
has no loopback TCP (`hostLoopback: deny`, ADR-0038), and `HTTPS_PROXY` cannot add a
header to a TLS tunnel without intercepting it. Can a sandboxed `claude` reach a proxy
that adds the token, with no TLS interception, and does Claude Code accept a dummy token
and a different base URL?

## Hypothesis and predictions
If Claude Code honours `ANTHROPIC_BASE_URL` with plain HTTP and sends the OAuth token as
a header the proxy can replace, then a proxy that rewrites that header and forwards over
TLS to api.anthropic.com runs a headless turn with the dummy inside. The transport is the
open part: loopback TCP is refused in the sandbox, so we expect to need either a unix
socket (if the base URL or a proxy setting accepts one), an exception for one loopback
port in the seatbelt profile, or a non-loopback address of the node. If none works, the
proxy has to intercept TLS with its own CA trusted inside the sandbox, or the token stays
in the sandbox.

## Plan
1. On the host, no sandbox: a small forwarding proxy that replaces the auth header with
   the real token. Run `claude -p` with a dummy `CLAUDE_CODE_OAUTH_TOKEN` and
   `ANTHROPIC_BASE_URL=http://127.0.0.1:<port>`; record whether the turn runs, which
   header carries the token, and any other host it calls (telemetry, the usage endpoint).
2. The same inside the seatbelt sandbox: first as is (expect loopback refused), then with
   each candidate transport: a unix-socket base URL, one allowed loopback port, the node's
   tailnet address.
3. With egress denied except the proxy, check that the session still works and that
   nothing inside can read the real token.

## Success criteria
- A turn and a resume run inside the sandbox with only a dummy token, through a named
  transport, with egress limited to the proxy; or a clear account of why not and which
  fallback is left.

## Runs
One line per run after its link, what was tried and what it showed; `yajna validate`
rejects a bare link here. Metric runs also land in results.tsv.

## Conclusion
Written once, by a human, when status becomes `concluded`.
