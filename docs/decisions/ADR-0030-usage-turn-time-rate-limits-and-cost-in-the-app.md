# ADR-0030: Usage, turn time, rate-limit utilisation and cost come from the daemon's events and the usage endpoint

Status: accepted
Date: 2026-10-25

## Context
The owner wants to see, in the app, each turn's time and tokens, the session's totals,
the account's 5-hour and 7-day rate-limit utilisation, and a cost in dollars. The daemon
already records usage once per assistant message id as `usage` events, by model
(EXP-20261016-rq1402), and Claude Code writes a `turn_duration` system line per turn. The
utilisation is on `GET api.anthropic.com/api/oauth/usage` with the OAuth access token and
the `oauth-2025-04-20` beta header, in 300 ms; Claude Code's own cached copy is refreshed
only by `/usage`. The token lives in the keychain (`Claude Code-credentials`) on macOS
and in `.credentials.json` on Linux; EXP-20261016-rq1408 showed that copying it into a
second store revokes the login, while reading it does not. The alternatives for the
utilisation were Claude Code's cached value (stale unless the person runs `/usage`), or
nothing. For cost, a price table per model in the hub, or none for a subscription
account.

## Decision
- Per turn: the core pairs each assistant turn with its `turn_duration` and the `usage`
  events of its messages and shows time and tokens under the reply.
- Per session: the hub sums usage by model per session and sends it with the session; the
  app shows it in the session's header.
- Rate limits: the daemon reads the access token from the credential store, read only
  and never written, logged or sent anywhere but the usage endpoint, calls the endpoint
  once a minute while a session is live, and reports the utilisation as health metrics
  (`claude.five_hour_pct`, `claude.seven_day_pct`) so the app charts them per node like
  CPU. [[EXP-20261025-25f6vm]] checks that reading the keychain item does not disturb the
  login and that the endpoint answers for this account.
- Cost: a price table per model in the hub's configuration, applied to the session's
  tokens; shown as an estimate.
[[EXP-20261025-25f8mm]] checks the turn and total numbers against Claude Code's `/cost`.

## Consequences
The daemon touches the owner's credential for the first time, read only; the sys crate
boundary for the keychain is small and the token never leaves the process except as a
bearer to Anthropic. A price table ages and has to be kept by hand. The usage endpoint's
scope requirement on other account kinds is unknown (HANDOFF section 5).
