#!/usr/bin/env bash
# RUN-20261016-rq1443: headless `claude -p --permission-mode default` with the hook server
# holding PermissionRequest for 5 s and then returning {}.
# Written on 2026-11-02 from the run's notes and the captured outputs, not run at the time.
# From the repo root (claude must be on PATH and logged in):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq1443/run.sh
# Writes events.jsonl, server.log, stream.jsonl and stderr.txt to out/ beside this script;
# the session runs in work/ beside it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike="$PWD/spikes/01-hooks"
out="$here/out"; work="$here/work"
rm -rf "$out" "$work"; mkdir -p "$out" "$work"
for v in $(compgen -e | grep '^CLAUDE' || true); do unset "$v"; done

python3 "$spike/hook_server.py" "$out" 7779 5 > "$out/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
sleep 1

cd "$work"
claude -p "Use the Bash tool to run exactly this command: touch exp01-marker.txt . Then stop." \
  --settings "$spike/settings-p.json" --permission-mode default \
  --output-format stream-json --verbose \
  > "$out/stream.jsonl" 2> "$out/stderr.txt" || true
grep -c permission_denied "$out/stream.jsonl" || true
