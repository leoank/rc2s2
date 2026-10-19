# ADR-0005: gRPC and protobuf on the wire, with separate node and control services

Status: accepted
Date: 2026-10-16

## Context
Protocol (question 5, the owner's answer in `../architecture-questions.md`): JSON over WebSocket, or gRPC with generated types for four kinds
of client. The owner noted that a client sees many daemons through the hub and needs more
than a node does.

## Decision
Protobuf with gRPC streams; `rc2s2.node.v1` for daemons and `rc2s2.control.v1` for clients,
sharing the event types in `rc2s2.event.v1`.

## Consequences
One bidirectional `Link` stream per node (named so because `Connect` collides with tonic's
generated client constructor). Kotlin types come from the same files.
