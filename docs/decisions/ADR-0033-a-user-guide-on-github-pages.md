# ADR-0033: A user guide on GitHub Pages, built by Nix with mdBook in yajna's theme

Status: accepted
Date: 2026-10-26

## Context
Everything written about rc2s2 so far is design: the architecture, the decision records
and the notebook. Someone setting it up, or the owner six months from now, has to read
the design to learn which flag does what. The owner asked for user docs published on
GitHub Pages, in the theme of the yajna guide. The alternatives were the README alone, a
`docs/user/` folder of markdown read on GitHub, or a built site.

## Decision
A user guide under `docs/guide/` (mdBook, the yajna guide's book.toml, CSS and sidebar
script), built by `nix build .#docs` and published to <https://leoank.github.io/rc2s2/>
by a GitHub Actions workflow on every push to main. `docs/guide/build.py` adds what must
not drift from the code: the command-line reference from `rc2s2 --help` and
`rc2s2-hub --help`, and the design section copied from `docs/architecture.md` and
`docs/decisions/`, with the notebook's `[[ID]]` links turned into links to the pages on
GitHub. It fails on a broken relative link, and `checks.docs` runs it in
`nix flake check`.

## Consequences
The guide describes behaviour, so a change a user would notice (a flag, a screen, a
notification) now has a page to update with it. The command-line reference and the
design section cannot fall behind. Building the site builds both release binaries, which
makes `nix build .#docs` and the Pages workflow take a few minutes from a cold cache.
