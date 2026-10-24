---
id: RUN-20261017-ebyrjv
previous_id: manthan/RUN-20261017-ho7k
type: run
schema_version: 1
kind: compute               # bench | compute | assembly | calibration
project: PRJ-0001
experiment: EXP-20261016-rq1402
protocols: []
status: done              # planned | in_progress | done | aborted
started: 2026-10-17T14:05+01:00
finished: 2026-10-17T14:08+01:00
commit: cc33196
created: 2026-10-17
author: agent:claude
---

## Notes

Direct call of the usage endpoint that [[RUN-20261017-dqeg0a]] found in the 2.1.302 code, with
the access token of the lease grant from [[RUN-20261017-dssg3d]] (read from its keychain item,
never printed). Script: `code/rc2s2-a-remote-command-and-control-session-server/spikes/02-transcript/usage_get.py`,
reads the credential JSON on stdin and prints the response with lists collapsed to their
first element. Output: `experiments/EXP-20261016-rq1402/run-usage-endpoint/usage-response-shape.txt`.

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <access token>
anthropic-beta: oauth-2025-04-20
Content-Type: application/json
```

Result: 200 in 303 to 365 ms, no rate-limit or retry-after headers on success. The body is
exactly the `utilization` object that `cachedUsageUtilization` stores: `five_hour` and
`seven_day` `{utilization (percent), resets_at (ISO), limit_dollars, used_dollars,
remaining_dollars, locked_reason}`, a `limits[]` list (`kind`, `group`, `percent`,
`severity`, `resets_at`, `scope`, `is_active`), `extra_usage`, `spend`,
`weekly_scoped_shares[]`, and a row of model- and feature-specific windows that were all
`null` for this account (`seven_day_opus`, `seven_day_sonnet`, ... plus codenamed ones).
Values at the time: five-hour 45 percent resetting 16:39Z, seven-day 18 percent.

So a daemon that holds a lease can poll this directly, every 60 s at most, and never needs
the `~/.claude.json` cache or a `claude -p "/usage"` probe for those accounts. For a person's
own interactive login on a node the daemon has no token and falls back to the probe.

## Checklist
- [x] endpoint answers a plain Bearer request with the `oauth-2025-04-20` beta header
- [x] body shape matches the on-disk cache
- [x] no token printed or stored
