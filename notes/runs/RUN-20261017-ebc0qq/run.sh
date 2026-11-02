#!/usr/bin/env bash
# RUN-20261017-ebc0qq: control for RUN-20261017-dssg3d. A refresh on the lease's own grant
# (copy C of slot A's pair, backdated) must leave a separate login untouched, the older pair in A
# must then fail and be cleared, and `claude auth logout` with C's config dir must revoke the
# grant at the server (a copy of C's access token saved before the logout gets 401). Uses
# run_lease.sh from experiments/EXP-20261016-rq1408/run-refresh/ and store.py from
# run-twosession/ (token hashes only); outputs go to out/ beside this script.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# The run's separate login was the owner's own; here it is a scratch config dir O, and slot A
# is a second scratch login instead of the pair left over from RUN-20261017-dssg3d. Both are
# signed in with `claude auth login` (browser sign-in by hand), logged out and their keychain
# items deleted at the end. The saved access token is held in a shell variable for one curl
# and never printed or written to disk.
# From the repo root, no dev shell needed (macOS, `claude`, python3 and curl on PATH):
#   notes/runs/RUN-20261017-ebc0qq/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
e=notes/experiments/EXP-20261016-rq1408
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT
out="$here/out"
mkdir -p "$out"
s="$(mktemp -d /private/tmp/rc2s2-control.XXXXXX)"
O="$s/O" A="$s/A" C="$s/C"
mkdir "$O" "$A" "$C"
svc() { printf 'Claude Code-credentials-%s' "$(printf %s "$1" | shasum -a 256 | cut -c1-8)"; }
lease() { bash "$e/run-refresh/run_lease.sh" "$1" "$out/$2"; }
state() { for d in "$@"; do python3 "$e/run-twosession/store.py" state "$d"; done; }
cleanup() {
  for d in "$O" "$A" "$C"; do CLAUDE_CONFIG_DIR="$d" claude auth logout >/dev/null 2>&1 || true; done
  for d in "$O" "$A" "$C"; do security delete-generic-password -s "$(svc "$d")" >/dev/null 2>&1 || true; done
  rm -rf "$s"
}
trap cleanup EXIT

# by hand: complete two browser sign-ins, O (stands in for the owner) and slot A (the lease grant)
CLAUDE_CONFIG_DIR="$O" claude auth login
CLAUDE_CONFIG_DIR="$A" claude auth login

(umask 077; security find-generic-password -s "$(svc "$A")" -w > "$C/.credentials.json")
python3 "$e/run-twosession/store.py" backdate "$C"
{
  echo "== setup"; state "$O" "$A" "$C"
  echo "== $(date +%T) lease C (expired access token, A's refresh token)"; lease "$C" C-run1.json
  echo "== state: new pair in C's hashed item, C's file gone; O unchanged"; state "$C" "$O"
  echo "== $(date +%T) lease A (now superseded)"; lease "$A" A-run3.json
  echo "== state: A cleared"; state "$A"
  echo "== $(date +%T) O's own login"; lease "$O" O-run1.json
} 2>&1 | tee "$out/timeline.log"

tok="$(security find-generic-password -s "$(svc "$C")" -w \
  | python3 -c 'import json, sys; print(json.load(sys.stdin)["claudeAiOauth"]["accessToken"])')"
{
  echo "== $(date +%T) logout with C's config dir"; CLAUDE_CONFIG_DIR="$C" claude auth logout || true
  echo "keychain item for C: $(security find-generic-password -s "$(svc "$C")" >/dev/null 2>&1 && echo PRESENT || echo deleted)"
  # the notes give only the path GET /api/oauth/usage; the host and beta header are assumed
  echo "== GET /api/oauth/usage with the saved copy: $(printf 'Authorization: Bearer %s\n' "$tok" \
    | curl -s -o /dev/null -w '%{http_code}' -H @- -H 'anthropic-beta: oauth-2025-04-20' https://api.anthropic.com/api/oauth/usage)"
  echo "== $(date +%T) O's own login again"; lease "$O" O-run2.json
} 2>&1 | tee -a "$out/timeline.log"
unset tok
