#!/usr/bin/env bash
# RUN-20261017-dssg3d: force an OAuth refresh inside a lease by backdating expiresAt, and watch
# rotation and revocation: lease A (backdated) refreshes, the source's old pair stops working,
# a fresh login on the source is a separate grant that leaves A alive, and lease B (a second copy
# of the old pair) fails and wipes itself. Uses run_lease.sh from
# experiments/EXP-20261016-rq1408/run-refresh/ and store.py from run-twosession/ (token hashes
# only); outputs go to out/ beside this script.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# The run copied the owner's own keychain credential and its refresh revoked the owner's login.
# This script never touches it: the source is a scratch config dir signed in with
# `claude auth login` (browser sign-in by hand, twice), and every grant it makes is logged out
# and every keychain item it creates deleted at the end.
# From the repo root, no dev shell needed (macOS, `claude` and python3 on PATH):
#   notes/runs/RUN-20261017-dssg3d/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
e=notes/experiments/EXP-20261016-rq1408
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT
out="$here/out"
mkdir -p "$out"
s="$(mktemp -d /private/tmp/rc2s2-refresh.XXXXXX)"
src="$s/src" A="$s/A" B="$s/B"
mkdir "$src" "$A" "$B"
svc() { printf 'Claude Code-credentials-%s' "$(printf %s "$1" | shasum -a 256 | cut -c1-8)"; }
lease() { bash "$e/run-refresh/run_lease.sh" "$1" "$out/$2"; }
state() { for d in "$@"; do python3 "$e/run-twosession/store.py" state "$d"; done; }
cleanup() {
  for d in "$A" "$src"; do CLAUDE_CONFIG_DIR="$d" claude auth logout || true; done
  for d in "$src" "$A" "$B"; do security delete-generic-password -s "$(svc "$d")" >/dev/null 2>&1 || true; done
  rm -rf "$s"
}
trap cleanup EXIT

# by hand: complete the browser sign-in; this is the baseline pair A0/R0
CLAUDE_CONFIG_DIR="$src" claude auth login

for d in "$A" "$B"; do
  (umask 077; security find-generic-password -s "$(svc "$src")" -w > "$d/.credentials.json")
done
python3 "$e/run-twosession/store.py" backdate "$A"
{
  echo "== setup"; state "$src" "$A" "$B"
  echo "== $(date +%T) lease A (expired access token)"; lease "$A" A-run1.json
  echo "== state: A's file gone, new pair in A's hashed item; source item unchanged"; state "$src" "$A"
  echo "== $(date +%T) source on the old pair (the run's subagents died here)"; lease "$src" src-run1.json
} 2>&1 | tee "$out/timeline.log"

# by hand: sign the source in again (the owner's /login in the run); a new grant A2/R2
CLAUDE_CONFIG_DIR="$src" claude auth login
{
  echo "== source after the new login"; state "$src"
  echo "== $(date +%T) lease A again (A1 from its hashed item)"; lease "$A" A-run2.json
  echo "== $(date +%T) lease B (copy of A0/R0)"; lease "$B" B-run1.json
  echo "== state: B cleared"; state "$B"
  echo "== $(date +%T) auth status with A's config dir"; CLAUDE_CONFIG_DIR="$A" claude auth status || true
} 2>&1 | tee -a "$out/timeline.log"
