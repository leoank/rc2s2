# ADR-0032: The hub pushes an alert when a rate-limit window is used too fast, nearly used up or used up

Status: accepted
Date: 2026-10-25

## Context
The node reports the account's rate-limit windows as health (ADR-0030): the 5-hour and
7-day windows and the Fable week, each with the percent used. Seeing that on a chart
needs the app open. The owner wants to hear when the limit is being used up very quickly,
or is already nearly gone early in the week, depending on when the window next resets.
The alternatives for where the alert comes from were the hub pushing it as it pushes held
requests, a banner in the app only, or both; for when it fires, a pace rule, fixed levels
and the limit itself. The owner chose the hub's push and all three rules (2026-10-25).

## Decision
The node reports each window's reset time with its percent (`usage.<window>_resets_s`,
Unix seconds), from the endpoint's `resets_at`, for both the live poll and Claude Code's
cached value. On every health sample the hub checks each window whose reset is still
ahead:

- **at the limit**: 100% used, priority 5, with the time to the reset;
- **near the limit**: 95% or 80% used (the higher level that applies), priority 4;
- **too fast**, for the 7-day windows only: at least 20% used and, at the rate since the
  window began (its length minus the time to the reset), the rest would be used before the
  reset; priority 4, with the time left and the time to run out. A 5-hour window spent in
  one sitting is how it is meant to be used, so its pace is not watched.

Each alert is sent once per window, reset time (to the minute) and rule, remembered in the
hub's `usage_alerts` table, so neither the next sample nor a second node on the same
account repeats it, and a hub restart does not either. The kind is `usage`, switchable with
`--no-notify usage` (ADR-0029). A tap opens the node's health screen.

## Consequences
The thresholds are constants in the hub, set by judgement, not use; they change in code.
Two accounts on different nodes whose windows reset in the same minute would share an
alert key and one of them would go unannounced; one account is the case today. A cached
value can be hours old, so an alert from it may come late, never wrong about the window it
names. The rate is an average since the window began, so a heavy first day raises a pace
alert that a quieter rest of the week would have made needless; that is the alert asked
for.
