---
id: AGS-20261107-7y7sf7
type: agent_session
schema_version: 1
title: "Phase 3 item 1.5: neusis writes the node's launch profiles and gives the sandbox's token as an agenix secret"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-07T07:13+00:00
finished: null
created_entities: []
edited_entities: []
outcome: pending          # done | failed | aborted | pending
created: 2026-11-07
author: agent:claude
---

## Task

The owner asked to start item 1.5 of [[PHS-20261106-3702q3]]: neusis's node module gains
the launch profiles, and the sandbox's Claude token comes from an agenix secret.

## What happened

- Before this, at the owner's request: 1.4 committed, rc2s2 pushed so neusis could pin
  it, neusis's rc2s2 input updated (uncommitted; the owner runs the neusis checks).
- neusis `homeModules/rc2s2-node.nix`: a `profiles` option (attrs of TOML values), written
  with `pkgs.formats.toml` as `{ profiles = ...; }` and passed as `--profiles <store
  file>` when not empty. Module test: no `--profiles` by default, the store file before
  the extra arguments with one.
- neusis `users/ank` rc2s2-node bundle: the profile `sandbox`, the one checked live in
  [[AGS-20261107-413qp0]] with its state directory moved to
  `~/.local/state/rc2s2/sandbox`, and the home secret `rc2s2SandboxClaudeToken` at
  `~/.config/rc2s2/sandbox-claude-token` as its `token_file`. rogue's machine test checks
  both.
- Not done, the owner's: the secret's `.age` file (`agenix edit
  modules/secrets/ank/rc2s2SandboxClaudeToken.age` with the token, then `agenix rekey`);
  until it exists rogue does not evaluate. No neusis check was run, at the owner's word.
- Decided without asking: a new secret path rather than the hand-made
  `~/.config/rc2s2/claude-oauth-token`, which agenix's link would collide with; that file
  can go once the secret works. The guide's home-manager paragraph, which still said the
  module leaves the hooks alone, now names `claudeHooks` and `profiles`.
