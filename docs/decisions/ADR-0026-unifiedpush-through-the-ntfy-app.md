# ADR-0026: The phone receives pushes through UnifiedPush, with the ntfy app as distributor

Status: accepted
Date: 2026-10-19

## Context
ADR-0008 and ADR-0025 have the hub publish to a self-hosted ntfy. The phone still needs a
way to receive. The alternatives: the person subscribes the ntfy app to the hub's topic
and a tap opens rc2s2; rc2s2 keeps its own foreground service streaming the topic; or
UnifiedPush, where the ntfy app is the distributor and rc2s2 registers an endpoint with
the hub. The owner chose UnifiedPush.

## Decision
The control service gains `RegisterPush`; the hub keeps each client's endpoint in
`push_endpoints` and posts a JSON message (kind, node, pending id, tool, title, body) to
every endpoint for each held request and finished turn, beside the optional ntfy topic,
and forgets an endpoint that answers 404 or 410. The app uses the UnifiedPush connector
(3.0.10): after connecting it registers with the phone's distributor, gives the endpoint to
the hub, and shows each message as its own notification, with Allow and Deny on a
permission request that act through the core without opening the app.

## Consequences
Decisions can be made from the notification. The phone needs a distributor installed (the
ntfy app pointed at the hub's ntfy). Messages go to the endpoint in plain text: the
connector's WebPush encryption needs the hub to encrypt per endpoint, deferred while the
ntfy server is the owner's own. A notification is not withdrawn when its request is
resolved elsewhere.
