# ADR-0003: One daemon per node keeps an outbound link to one hub on the tailnet; clients talk only to the hub

Status: accepted
Date: 2026-10-16

## Context
Topology (question 3, the owner's answer in `../architecture-questions.md`). The hub could live on a home server, a VPS or a machine on the
tailnet.

## Decision
A small Rust daemon per machine collects hooks, transcripts and health and holds one
outbound stream to the hub; the hub runs on a machine on the owner's tailnet; clients see the
hub's view across nodes, never a node directly.

## Consequences
Nodes need no inbound port; the hub is the single place for records, pending requests and
secrets.
