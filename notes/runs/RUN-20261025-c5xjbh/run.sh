#!/usr/bin/env bash
# RUN-20261025-c5xjbh: plan step 3 of EXP-20261025-25f6vm. Step 2's script
# (RUN-20261025-c35jpc) left polling the real item and endpoint once a minute across an
# access-token refresh: from now until UNTIL (UTC, default 13:15, after the token's expiry at
# 12:59Z on 2026-10-25). A refresh shows as a new token hash and expiry and a new mdat in
# mdat.txt; it happens only when some Claude Code session makes a request after the expiry.
# From the repo root:
#   nix develop -c notes/runs/RUN-20261025-c5xjbh/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
DURATION_S=$(python3 -I -c 'import sys, datetime as d; n = d.datetime.now(d.timezone.utc)
h, m = map(int, sys.argv[1].split(":")); print(int((n.replace(hour=h, minute=m, second=0) - n).total_seconds()))' "${UNTIL:-13:15}") \
  OUT="$here" exec "$here/../RUN-20261025-c35jpc/run.sh"
