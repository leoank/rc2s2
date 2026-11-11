# ADR-0040: The project is renamed saarthi, "Guiding agents to victory", all the way down

Status: accepted
Date: 2026-11-11

## Context
On 2026-11-06 the owner chose a new name, saarthi (Sanskrit for charioteer), and the app
icon became a chariot wheel; everything else still said rc2s2. On 2026-11-11 they asked
for the rename with the tagline "Guiding agents to victory" and chose
([[AGS-20261111-ffkhjn]]): rename everything, the protocol included; a new Android app id;
new default paths with no fallback to the old ones; and the code moved to the existing
GitHub repo `leoank/saarthi`, whose contents (a skills scaffold from May) go to a branch
`old-code` first.

## Decision
- Binaries `saarthi` (node, `run`, `ctl`) and `saarthi-hub`; crates `saarthi-proto`,
  `saarthi-node`, `saarthi-hub`, `saarthi-core`; Nix packages and checks likewise.
- Protocol packages `saarthi.event.v1`, `saarthi.node.v1`, `saarthi.control.v1`. The hub,
  the nodes and the app must be updated together: the gRPC paths change.
- The app is `me.leoank.saarthi`, labelled saarthi; the client core's bindings are
  `uniffi.saarthi_core`.
- Default paths: `~/.config/saarthi` (`ctl.toml`, token files),
  `~/.local/state/saarthi` (`hook.sock`, sandbox state in the examples); environment
  variables `SAARTHI_HUB` and `SAARTHI_TOKEN_FILE`. Old paths are not read.
- The guide moves to `https://leoank.github.io/saarthi/`; the notebook's namespace, the
  name other repos link to it by, is `saarthi`.
- The tagline goes on the README, the guide, the flake and the project page.

Settled by the agent, for the owner to revisit:
- Records stay as written: accepted ADRs, notebook pages (sessions, experiments, runs,
  logs, phase items already done) and spikes keep saying rc2s2, as history. Living
  documents (README, guide, architecture, HANDOFF, the project page's design) say saarthi.
- Links to the neusis modules keep their current file names (`rc2s2-node.nix`,
  `rc2s2-hub.nix`) until neusis renames them.

## Alternatives considered
- Rename only what people see, keeping crate and protocol names: old and new parts would
  keep talking, but the code would carry two names for good.
- Keep the app id: an in-place update with its settings kept, but the id would say rc2s2
  forever.
- Fall back to the old paths for a while: smoother, but code to remove later, and neusis
  sets the paths explicitly anyway.
- Rename the GitHub repo rc2s2: GitHub redirects it, but the name saarthi was taken by the
  owner's own scaffold.

## Consequences
- A coordinated switch: the hub, every node and the app move together, and the old app is
  uninstalled after the new one has its hub address and token and has registered for push.
- neusis must rename its modules' packages, binary paths, launchd label and service paths,
  and point its flake input at `github:leoank/saarthi`; existing token files and state move
  from `rc2s2` to `saarthi` directories, or the module keeps naming the old paths.
- Sessions started under the old wrapper (`rc2s2 run`) keep running but cannot reach the
  new node: its socket path changed. Restart them under `saarthi run`.
- Links from manthan (`[[rc2s2/ID]]`) break until manthan's `[repos]` names `saarthi`.
