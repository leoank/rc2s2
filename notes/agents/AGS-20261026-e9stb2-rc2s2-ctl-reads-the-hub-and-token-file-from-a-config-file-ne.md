---
id: AGS-20261026-e9stb2
type: agent_session
schema_version: 1
title: rc2s2 ctl reads the hub and token file from a config file; network charts on the health screen
model: "claude-opus-5-5"
harness: "claude-code"
started: 2026-10-26T13:01+00:00
finished: 2026-10-26T13:01+00:00
created_entities:
  - AGS-20261026-e9stb2
edited_entities:
  - PRJ-0001
outcome: done          # done | failed | aborted | pending
created: 2026-10-26
author: agent:claude
---

## Task

Two from the owner: `rc2s2 ctl` should not need the hub address and token on every call
(their item in `tasks.md`), and rogue's network metrics, listed in the guide, do not show
in the app.

## What happened

- **ctl config.** `rc2s2 ctl` reads `ctl.toml` from `$XDG_CONFIG_HOME/rc2s2/`, else
  `~/.config/rc2s2/`, with `hub` and `token_file` (a leading `~/` expanded); `--hub` and
  `--token-file` and their variables `RC2S2_HUB` and `RC2S2_TOKEN_FILE` still win, and
  with neither the error names the file. Unknown keys are an error. `toml` 1.1.8, already
  in the lock as a transitive dependency, is now a direct one. Unit tests for the
  precedence, `~`, the missing file and a misspelt key; checked against a scratch hub with
  no config (the error), a config, and the variable winning.
- **Network.** Not a node bug: the node sends `net.rx_bytes` and `net.tx_bytes` (bytes
  since its previous sample) every sample, but the app never charted them; its
  `Series.label` already had an unused "MB/s" unit. The health screen now has
  "Network in" and "Network out" in MB/s, each the bytes over the time since the previous
  sample that carries the metric, so the usage poller's own samples in between do not
  shorten the interval. Checked on the emulator against a scratch hub and node sampling
  every 5 s: both charts render with rates and peaks (screenshot in the session's
  scratchpad, not kept).
- Guide (`using/ctl.md`, the quickstart, `setup/hub.md`, `using/health.md`,
  `reference/metrics.md`, the introduction) and `docs/architecture.md` updated.
- Decided without asking: the file's name and place, TOML, the token kept in its own file
  (no inline `token` key), and two charts rather than one with two lines.

