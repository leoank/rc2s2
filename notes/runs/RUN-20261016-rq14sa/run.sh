#!/usr/bin/env bash
# RUN-20261016-rq14sa: tests (a) and (b) of EXP-20261016-rq1407 with the mxc spike binary:
# /bin/echo under an empty seatbelt policy, then curl to a host 127.0.0.1:18765 listener with
# ingress+hostLoopback allow (reaches it) and with egress allow only (refused, the hostLoopback
# trap). Uses the policy files in the experiment dir; logs go beside this script.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# From the repo root, in the spike's dev shell (macOS, port 18765 free):
#   nix develop ./spikes/07-mxc -c \
#     notes/runs/RUN-20261016-rq14sa/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/07-mxc
exp=notes/experiments/EXP-20261016-rq1407

cargo build --quiet --manifest-path "$spike/Cargo.toml"
m="$spike/target/debug/mxc-spike"
tmp="$(mktemp -d)"
srv=
trap 'kill "$srv" 2>/dev/null || true; rm -rf "$tmp"' EXIT

# (a)
"$m" "$exp/policy-echo.json" -- /bin/echo hi 2>&1 | tee "$here/test-a-echo.log" || true

# (b) host listener outside the sandbox
echo hello-from-host > "$tmp/index.html"
python3 -m http.server 18765 --bind 127.0.0.1 --directory "$tmp" 2> "$tmp/http.log" &
srv=$!
sleep 1
"$m" "$exp/policy-curl.json" -- /usr/bin/curl -sS --max-time 5 http://127.0.0.1:18765/ 2>&1 \
  | tee "$here/test-b-curl.log" || true
"$m" "$exp/policy-curl-egress-only.json" -- /usr/bin/curl -sS --max-time 5 http://127.0.0.1:18765/ 2>&1 \
  | tee "$here/test-b2-curl-egress-only.log" || true
