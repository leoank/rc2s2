# ADR-0018: Agent credentials are grants the hub leases to nodes as slots

Status: accepted
Date: 2026-10-17

## Context
A Claude OAuth refresh rotates both tokens and revokes the old pair (EXP-20261016-rq1408): two stores
holding one grant kill each other, while many sessions on one store are fine.

## Decision
An account is a set of slots, each one grant from its own login; a slot is leased to one
node at a time and written into a slot directory that every session of that account on
the node uses.

## Consequences
Details in "Credentials" of the architecture. One run is still pending
(RUN-20261017-m3gg0y).
