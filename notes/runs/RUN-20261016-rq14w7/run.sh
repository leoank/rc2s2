#!/usr/bin/env bash
# RUN-20261016-rq14w7: lease a credential into a temporary config dir and answer one headless
# turn from it, then check the source and the leased file are unchanged, no keychain item
# appeared for the lease dir, and the source login still answers. Writes lease.json and
# state.log to out/ beside this script (token hashes only, never a token).
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# The run leased the owner's own keychain credential "Claude Code-credentials". This script
# never touches it: the source is a scratch config dir signed in with `claude auth login` (a
# fresh grant, browser sign-in by hand), logged out and removed at the end.
# From the repo root, no dev shell needed (macOS, `claude` and python3 on PATH):
#   notes/runs/RUN-20261016-rq14w7/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
h=notes/experiments/EXP-20261016-rq1408/run-twosession
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT
out="$here/out"
mkdir -p "$out"
src="$(mktemp -d /private/tmp/rc2s2-src.XXXXXX)"
lease="$(mktemp -d /private/tmp/rc2s2-lease.XXXXXX)"
svc() { printf 'Claude Code-credentials-%s' "$(printf %s "$1" | shasum -a 256 | cut -c1-8)"; }
cleanup() {
  CLAUDE_CONFIG_DIR="$src" claude auth logout || true
  for d in "$src" "$lease"; do security delete-generic-password -s "$(svc "$d")" >/dev/null 2>&1 || true; done
  rm -rf "$src" "$lease"
}
trap cleanup EXIT

# by hand: complete the browser sign-in; the grant lands in the keychain item for $src
CLAUDE_CONFIG_DIR="$src" claude auth login

(umask 077; security find-generic-password -s "$(svc "$src")" -w > "$lease/.credentials.json")
{ echo "== before"; python3 "$h/store.py" state "$src"; python3 "$h/cred_state.py" "$lease/.credentials.json"; } \
  | tee "$out/state.log"

(cd "$lease" && CLAUDE_CONFIG_DIR="$lease" claude -p "Reply with just the word ok." --output-format json) \
  > "$out/lease.json" 2> "$out/lease.stderr" || true
python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print({k: d.get(k) for k in ("subtype","is_error","result")})' "$out/lease.json"

{ echo "== after"; python3 "$h/store.py" state "$src"; python3 "$h/cred_state.py" "$lease/.credentials.json"
  echo "keychain item for the lease dir: $(security find-generic-password -s "$(svc "$lease")" >/dev/null 2>&1 && echo PRESENT || echo none)"
  echo "lease dir contents:"; ls -A "$lease"
  echo "== source login still answers"
  (cd "$src" && CLAUDE_CONFIG_DIR="$src" claude -p "Reply with just the word ok.") || true
} 2>&1 | tee -a "$out/state.log"
