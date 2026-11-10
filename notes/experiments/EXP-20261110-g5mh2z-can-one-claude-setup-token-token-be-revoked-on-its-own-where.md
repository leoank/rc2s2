---
id: EXP-20261110-g5mh2z
type: experiment
schema_version: 1
title: Can one claude setup-token token be revoked on its own, where, and how fast does a running session holding it stop
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
ADR-0039 makes one token per node and account so a lost node can be cut off without
stopping the others, and counts on revocation as the answer to a leaked token. Can one
`setup-token` token be revoked on its own? Where (claude.ai settings, an API, a CLI
command)? Does revoking it leave the account's other tokens and the owner's login working,
and how fast does a session holding it stop?

## Hypothesis and predictions
If each `setup-token` run is its own grant (as [[EXP-20261107-413q6m]] suggests), it is
listed and revocable on its own and only sessions holding it fail, within one request. If
tokens can only be revoked all together, or not at all before they expire, per-node tokens
buy nothing, and the proxy (so sandboxes never hold the token) becomes a precondition for
giving tokens to sandboxes at all.

## Plan
1. The owner makes two throwaway tokens, A and B. Look for where they are listed:
   claude.ai's settings, `claude auth` subcommands, `claude setup-token --help`.
2. Start one headless session with A and one with B, a turn each.
3. The owner revokes A. Send both sessions a turn at once, then every 30 s for five
   minutes; record when A's session fails and with what error, and that B's and the owner's
   own sessions keep working.
4. Revoke B too, so no test token is left alive.

## Success criteria
- A is revoked without touching B or the owner's login, and A's session fails on its next
  request; the place and steps to revoke are written down for the guide.

## Runs
One line per run after its link, what was tried and what it showed; `yajna validate`
rejects a bare link here. Metric runs also land in results.tsv.

## Conclusion
Written once, by a human, when status becomes `concluded`.
