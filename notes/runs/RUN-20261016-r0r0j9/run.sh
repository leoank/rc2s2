#!/usr/bin/env bash
# RUN-20261016-r0r0j9: the structure-only pass over every Claude Code transcript on this machine.
# Written on 2026-11-02 from the run's notes, not run at the time. Runs `shapes.py` (stdlib
# only) over ~/.claude/projects, read only, and writes the redacted report to shapes.md beside
# this script; the captured report in the experiment sidecar is left alone.
# From the repo root (the run used the host python3; the project shell has one too):
#   nix develop . -c \
#     notes/runs/RUN-20261016-r0r0j9/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/02-transcript

python3 "$spike/shapes.py" > "$here/shapes.md"

# by hand: the cross-checks in the notes (usage summed after de-duplicating on message.id
# against cost-state on throwaway sessions, the ~/.claude.json backups' fetchedAtMs, and the
# key sets of the ~/.claude state files) were done interactively; .credentials.json is not read.
echo "report in $here/shapes.md; check it holds no paths, mail or workspace names before keeping it"
