# ADR-0008: Push notifications go through a self-hosted ntfy

Status: accepted
Date: 2026-10-16

## Context
Waking the phone when a session blocks (question 8, the owner's answer in `../architecture-questions.md`): FCM, self-hosted ntfy or
UnifiedPush, or a foreground service.

## Decision
Self-hosted ntfy on the hub machine.

## Consequences
No Firebase project; the PermissionRequest itself is the trigger, since the Notification
hook arrives only after it returns (EXP-0001).
