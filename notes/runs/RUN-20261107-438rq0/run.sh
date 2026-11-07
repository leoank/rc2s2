#!/usr/bin/env bash
# RUN-20261107-438rq0 (EXP-20261107-413q82): a stream-json claude session under mxc seatbelt.
# From the repo root. Needs the owner's long-lived token from `claude setup-token` in
# ~/.config/rc2s2/claude-oauth-token (EXP-20261107-413q6m); it is never printed.
set -euo pipefail
here=notes/runs/RUN-20261107-438rq0
bridge=$(nix build --no-link --print-out-paths "path:$PWD/spikes/07-mxc")/bin/stream
python3 -I "$here/driver.py" "$bridge" "$HOME/.config/rc2s2/claude-oauth-token" "$here/out" | tee "$here/run.log"
