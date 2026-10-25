---
id: AGS-20261025-2vf58e
type: agent_session
schema_version: 1
title: "Phase 1.5: commit neusis, deploy ghost001, then item 7, the prompt bar"
model: "claude-fable-5-1"
harness: "claude-code"
started: 2026-10-25T02:36+00:00
finished: 2026-10-25T02:50+00:00
created_entities:
  - AGS-20261025-2vf58e
edited_entities:
  - PRJ-0001
outcome: done             # done | failed | aborted | pending
created: 2026-10-25
author: agent:claude
---

## Task

Commit the neusis changes, deploy ghost001, then phase 1.5 item 7 (the prompt bar). The
owner had already switched rogue.

## What happened

- **The rogue switch had broken the node.** neusis's lock still pinned rc2s2 at `a709cab`,
  so the switched launchd agent ran a daemon that does not know `--ignore-cwd`; it printed
  the usage error and KeepAlive restarted it in a loop. Fix: rc2s2 `main` pushed to GitHub
  (18 commits, needed for the lock), `nix flake update rc2s2` in neusis to `ce1281e`, and
  one neusis commit (`bee354c`) with the module option, the rogue wiring, the test and the
  lock, its body saying why the lock moves with it. `darwinConfigurations.rogue.system`
  builds with the new lock. By 02:50 the owner had switched rogue again: the daemon runs
  with `--ignore-cwd */target/live-work-*` and registered with the new hub.
- **ghost001.** `neusis cloud deploy ghost001` needs `nixos-rebuild`, which is in the
  neusis dev shell, not on the host PATH; run from `nix develop`, it built the new hub on
  the remote builder and switched the host (see the closing log line for the outcome).
- **Item 7.** The prompt bar is a pill (`BasicTextField`, no outline) with Send and Stop
  inside it, a placeholder that says "Queue a prompt" while Claude works, and one line
  above it on the session's state where it changes what Send does: waiting for you (answer
  the card above), working (Send queues, Stop interrupts), started outside the wrapper
  (a peer message, no stop). The person's turns are tinted with the primary container
  behind the accent bar. Claude's text keeps single line breaks instead of joining them
  into a paragraph, as the terminal does. Checked on the emulator in the idle, working,
  typed and waiting states.
- **Decisions made without asking:** the state line's wording; the pill shape; no change
  to the top bar's subtitle, which the usage now makes long enough to be clipped on a
  narrow screen (the Usage dialog has the detail).
