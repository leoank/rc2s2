#!/usr/bin/env bash
# RUN-20261019-an0gmm: how many transcript lines a node produces and whether their uuids are
# unique within a session and across sessions. Reads ~/.claude/projects (or
# CLAUDE_CONFIG_DIR) read only and writes counts to scan.txt beside this script; no content,
# paths or ids are written.
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261019-an0gmm/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
python3 -I "$here/scan.py" "${CLAUDE_CONFIG_DIR:-$HOME/.claude}" | tee "$here/scan.txt"
