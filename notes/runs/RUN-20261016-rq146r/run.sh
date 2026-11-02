#!/usr/bin/env bash
# RUN-20261016-rq146r: the AskUserQuestion prompt in headless `claude -p`, hook server in pre and perm mode.
# Written on 2026-11-02 from the run's notes, not run at the time. For each mode it starts
# hook_server.py (port 7791 pre, 7792 perm, hold 5 s), runs one `claude -p` turn with the
# matching settings file in a scratch cwd, and keeps events, server log, stream and stderr in
# run-pre/ and run-perm/ beside this script. Expected: the model reports the tool is missing.
# From the repo root, with a logged-in `claude` on PATH:
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq146r/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
hooks="$(pwd)/spikes/01-hooks"
prompt='Use the AskUserQuestion tool to ask me one question: which drink I prefer, with exactly two options labelled Tea and Coffee. After the tool returns, reply with exactly the label I chose and nothing else.'
work="$(mktemp -d)"
srv=
trap 'kill $srv 2>/dev/null; rm -rf "$work"' EXIT

for mode in pre perm; do
  port=7791; [ "$mode" = perm ] && port=7792
  out="$here/run-$mode"
  rm -rf "$out"; mkdir -p "$out"
  echo "$mode" > "$out/ask_mode"
  python3 "$hooks/hook_server.py" "$out" "$port" 5 > "$out/server.log" 2>&1 &
  srv=$!
  sleep 1
  (cd "$work" && claude -p "$prompt" --settings "$hooks/settings-ask-$mode.json" \
    --output-format stream-json --verbose < /dev/null > "$out/stream.jsonl" 2> "$out/stderr.txt") || true
  kill "$srv"; wait "$srv" 2>/dev/null || true
done
