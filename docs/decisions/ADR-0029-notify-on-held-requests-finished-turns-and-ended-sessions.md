# ADR-0029: The hub notifies on held requests, finished turns and ended sessions, each kind switchable

Status: accepted
Date: 2026-10-25

## Context
ADR-0025 made the hub push one message per held request and per finished turn. In daily
use the finished turns of every session on the node reach the phone, which is noisy, but
the owner wants to use the app for a while before deciding what to cut; and a session that
ends, by exit or by a crash, gives no push today. The alternatives were to push finished
turns only for sessions touched from the phone (prompted, interrupted, or under the
wrapper), or to leave the policy as it is and add nothing.

## Decision
The hub pushes for a held request (permission or question), a finished turn, and a session
that ended, with the kind named in each message. Each kind can be switched off with
`--no-notify <kind>` on the hub, so the policy is tuned by configuration, not by code,
once the owner has used it. Whether a finished turn should push only for sessions touched
from the phone is left open on the project page until then.

## Consequences
One more message kind, `ended`, with the session's directory and whether it exited or was
killed; the app shows it without actions. The push payload carries `kind` already, so the
app needs no new parsing. The noise stays until the switches are set.
