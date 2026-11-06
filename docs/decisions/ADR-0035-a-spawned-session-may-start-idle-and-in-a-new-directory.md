# ADR-0035: A spawned session may start idle, and in a directory the spawn creates

Status: accepted
Date: 2026-11-06

## Context
ADR-0034's spawn took an existing directory and a required first prompt. Using it on the
phone, the owner could not start a session in a directory that did not exist yet, and
asked why the prompt had to be written in the dialog. The prompt was required only
because the first design sent it with the spawn: `claude -p` with stream-json input
starts, registers as `idle` and waits for its first line when given none, writing no
transcript until then (checked on Claude Code 2.1.318).

## Decision
The first prompt is optional. Without one the node starts the session idle, and the
person types the first prompt in the session's screen like any later one.

A spawn may ask to create its directory (`create`; on by default in the app's dialog,
`--create` for `rc2s2 ctl spawn`). The node makes the missing part of the path only when
its nearest existing parent, symlinks resolved, lies under a spawn root and the part to
create is plain names, no `.` or `..`; then it checks the made directory like any other.

## Consequences
A typo in the app's directory field makes a new directory under the root rather than
failing; the switch turns that off. An idle spawned session holds a process until it is
killed or its node daemon stops.
