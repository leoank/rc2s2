#!/usr/bin/env bash
# RUN-20261016-rq14zf: headless `claude -p` with hook events in the stream; a helper writes
# an allow to answer.json 8 s after the PermissionRequest arrives.
# Written on 2026-11-02 from the run's notes and the captured outputs, not run at the time.
# From the repo root (claude must be on PATH and logged in):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14zf/run.sh
# Writes events.jsonl, server.log, stream.jsonl and stderr.txt to out/ beside this script;
# the session runs in work/ beside it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike="$PWD/spikes/01-hooks"
out="$here/out"; work="$here/work"
rm -rf "$out" "$work"; mkdir -p "$out" "$work"
for v in $(compgen -e | grep '^CLAUDE' || true); do unset "$v"; done

python3 "$spike/hook_server.py" "$out" 7783 60 > "$out/server.log" 2>&1 &
server=$!
(
  until grep -q PermissionRequest "$out/events.jsonl" 2>/dev/null; do sleep 0.2; done
  sleep 8
  echo '{"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}}' \
    > "$out/answer.json"
) &
helper=$!
trap 'kill "$server" "$helper" 2>/dev/null || true' EXIT
sleep 1

cd "$work"
claude -p "Please run this shell command with the Bash tool and then stop: date > now.txt" \
  --settings "$spike/settings-p2.json" --permission-mode default \
  --output-format stream-json --verbose --include-hook-events \
  > "$out/stream.jsonl" 2> "$out/stderr.txt"
cat "$work/now.txt"
