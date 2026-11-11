---
id: AGS-20261111-ffkhjn
type: agent_session
schema_version: 1
title: "Rename the project rc2s2 to saarthi, Guiding agents to victory: code, protocol, app id, paths, docs and the repo"
model: claude-opus-5-5
harness: claude-code
started: 2026-11-11T14:05+00:00
finished: 2026-11-11T14:20+00:00
created_entities: []
edited_entities: [PRJ-0001]
outcome: done             # done | failed | aborted | pending
created: 2026-11-11
author: agent:claude
---

## Task
The owner asked to rename the project to saarthi with the tagline "Guiding agents to
victory". Their choices, asked in one round: everything including the protocol packages,
a new app id, new default paths with no fallback, and the code moved to the existing repo
`leoank/saarthi` after its contents go to a branch `old-code`.

## What happened
- Replaced rc2s2, Rc2s2 and RC2S2 in the crates, the app, the flake, the README, the guide,
  `docs/architecture.md`, `docs/architecture-questions.md` and `docs/HANDOFF.md`; moved
  `proto/rc2s2/`, the Kotlin package directory and the guide's stylesheet. Realigned three
  ASCII diagrams and wrapped two lines the longer name pushed past 100 columns.
- The tagline on the README, the guide's introduction and description, the flake, and
  the project page's title. ADR-0040 records the rename; the project page's open question
  now says what is done and what is left to the owner.
- Checked: `cargo test --workspace` (18 suites ok), clippy (only the two old warnings),
  `cargo fmt --check` (the 169 differences already on main, none new), `nix flake check`,
  the core for Android and the debug APK.
- Decided without asking, in ADR-0040: records keep rc2s2 as history; links to the neusis
  modules keep their file names until neusis renames them; manthan's directory path in
  `yajna.toml` is left alone; the notebook's namespace becomes saarthi.
