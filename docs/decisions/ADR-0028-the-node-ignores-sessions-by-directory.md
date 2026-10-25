# ADR-0028: The node ignores sessions whose working directory matches a pattern

Status: accepted
Date: 2026-10-25

## Context
The live checks start throwaway Claude Code sessions on the owner's own machine, under the
project's `target/`, with hooks pointed at a scratch node through `--settings`. The owner's
live node sees them anyway: through the registry, and since the hook block went into
`~/.claude/settings.json`, through every hook as well, because Claude Code merges the user's
hooks with the `--settings` ones. The live hub then lists them and pushes notifications
for their held requests and finished turns to the owner's phone. The alternatives were a
scratch `CLAUDE_CONFIG_DIR` with a login of its own (a second browser login, and a second
credential to keep away from the first, EXP-20261016-rq1408), or a hub rule that notifies
only for sessions started through a known wrapper (the tests would still be listed).

## Decision
`rc2s2 node serve --ignore-cwd <glob>`, repeatable: a session whose working directory
matches is not tracked at all, from the registry or from hooks, and its transcript is not
tailed. The owner's node module sets `*/target/live-work-*` for this machine. The run
scripts keep their scratch directories under that pattern.

## Consequences
Test sessions never reach the live hub, so no notification and no listing. A session the
owner starts in an ignored directory is invisible to rc2s2 as well, which the pattern must
keep in mind. The scratch node in a run script sets no ignore pattern, so it still sees
the test session.
