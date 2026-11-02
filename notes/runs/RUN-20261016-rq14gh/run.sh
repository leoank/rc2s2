#!/usr/bin/env bash
# RUN-20261016-rq14gh: headless `claude -p` with a 20 s per-hook timeout (settings-t20b.json)
# against a hook server holding PermissionRequest for 90 s, to measure when claude cancels.
# Written on 2026-11-02 from the run's notes and the captured outputs, not run at the time.
# From the repo root (claude must be on PATH and logged in):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14gh/run.sh
# Writes events.jsonl, server.log, stream.jsonl and stderr.txt to out/ beside this script;
# the session runs in work/ beside it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike="$PWD/spikes/01-hooks"
out="$here/out"; work="$here/work"
rm -rf "$out" "$work"; mkdir -p "$out" "$work"
for v in $(compgen -e | grep '^CLAUDE' || true); do unset "$v"; done

python3 "$spike/hook_server.py" "$out" 7781 90 > "$out/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
sleep 1

cd "$work"
claude -p "Please run this shell command with the Bash tool and then stop: date > now.txt" \
  --settings "$spike/settings-t20b.json" --permission-mode default \
  --output-format stream-json --verbose --include-hook-events \
  > "$out/stream.jsonl" 2> "$out/stderr.txt" || true
grep client_closed_during_hold "$out/events.jsonl"
