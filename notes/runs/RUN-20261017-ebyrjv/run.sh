#!/usr/bin/env bash
# RUN-20261017-ebyrjv: a direct GET of https://api.anthropic.com/api/oauth/usage with a lease grant's token.
# Written on 2026-11-02 from the run's notes, not run at the time. Reads the credential of a
# lease's scratch CLAUDE_CONFIG_DIR (logged in as in RUN-20261017-dssg3d) from its own keychain
# item "Claude Code-credentials-<sha256(dir)[:8]>" and pipes it straight into usage_get.py,
# which prints the status and the response shape; the token is never printed or stored. Never
# point it at the owner's own config dir. Output: usage-response-shape.txt beside this script.
# From the repo root, on macOS, with network access:
#   nix develop . -c \
#     notes/runs/RUN-20261017-ebyrjv/run.sh LEASE_CONFIG_DIR
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/02-transcript
lease_dir="${1:?usage: run.sh LEASE_CONFIG_DIR (the scratch CLAUDE_CONFIG_DIR of a lease grant)}"

service="Claude Code-credentials-$(printf '%s' "$lease_dir" | shasum -a 256 | cut -c1-8)"
security find-generic-password -s "$service" -w \
  | python3 "$spike/usage_get.py" > "$here/usage-response-shape.txt"
head -1 "$here/usage-response-shape.txt"
