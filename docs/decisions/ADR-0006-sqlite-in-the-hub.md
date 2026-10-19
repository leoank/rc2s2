# ADR-0006: SQLite in WAL mode inside the hub keeps the full event stream, with Postgres later

Status: accepted
Date: 2026-10-16

## Context
Storage and records (question 6, the owner's answer in `../architecture-questions.md`): which records per run, and SQLite or Postgres.

## Decision
SQLite in WAL mode embedded in the hub through sqlx, the full event stream, no retention
limit for now; Postgres later as a change of driver and migrations.

## Consequences
Queries are checked at run time so building needs no database. Deduplication must fit
SQLite's indexes (ADR-0020).
