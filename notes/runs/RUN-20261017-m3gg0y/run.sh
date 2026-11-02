#!/usr/bin/env bash
# RUN-20261017-m3gg0y: do many sessions on one credential store survive a refresh? (1) race:
# backdate the store, then two run_lease.sh at once; (2) idle sibling: idle_session.py answers
# turn 1, the store is backdated, one run_lease.sh refreshes, `go` is touched and turn 2 must
# pick up the new pair; (3) logout and delete the store. Uses the helpers in
# experiments/EXP-20261016-rq1408/run-twosession/ (store.py prints token hashes only); outputs
# go to out/ beside this script.
# Written on 2026-11-02 from the run's plan; the run is still planned, nothing was run.
# Needs a fresh grant: a scratch config dir signed in with `claude auth login` (browser sign-in
# by hand). Never point it at the owner's own credential; a copy of it would be revoked by the
# refresh (RUN-20261017-dssg3d).
# From the repo root, no dev shell needed (macOS, `claude` and python3 on PATH):
#   notes/runs/RUN-20261017-m3gg0y/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
h=notes/experiments/EXP-20261016-rq1408/run-twosession
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT
out="$here/out"
mkdir -p "$out"
cfg="$(mktemp -d /private/tmp/rc2s2-twosession.XXXXXX)"
idle=
cleanup() {
  kill "$idle" 2>/dev/null || true
  CLAUDE_CONFIG_DIR="$cfg" claude auth logout || true
  python3 "$h/store.py" delete "$cfg" || true
  rm -rf "$cfg"
}
trap cleanup EXIT

# by hand: complete the browser sign-in; the grant lands in the keychain item for $cfg
CLAUDE_CONFIG_DIR="$cfg" claude auth login

# 1. race
{
  echo "== race"; python3 "$h/store.py" state "$cfg"
  python3 "$h/store.py" backdate "$cfg"
  bash "$h/run_lease.sh" "$cfg" "$out/race-1.json" > "$out/race-1.log" &
  bash "$h/run_lease.sh" "$cfg" "$out/race-2.json" > "$out/race-2.log" &
  wait
  cat "$out/race-1.log" "$out/race-2.log"
  python3 "$h/store.py" state "$cfg"
} 2>&1 | tee "$out/timeline.log"

# 2. idle sibling
rm -f "$out/go"
python3 "$h/idle_session.py" "$cfg" "$out" > "$out/idle.log" 2>&1 &
idle=$!
until grep -q 'waiting for' "$out/idle.log"; do kill -0 "$idle"; sleep 1; done
{
  echo "== idle sibling"; cat "$out/idle.log"
  python3 "$h/store.py" backdate "$cfg"
  bash "$h/run_lease.sh" "$cfg" "$out/refresh.json"
  python3 "$h/store.py" state "$cfg"
} 2>&1 | tee -a "$out/timeline.log"
touch "$out/go"
wait "$idle" || true
idle=
cat "$out/idle.log" | tee -a "$out/timeline.log"

# 3. revoke the scratch grant and delete its keychain item
{
  echo "== logout"; CLAUDE_CONFIG_DIR="$cfg" claude auth logout || true
  python3 "$h/store.py" delete "$cfg"; python3 "$h/store.py" state "$cfg"
} 2>&1 | tee -a "$out/timeline.log"
