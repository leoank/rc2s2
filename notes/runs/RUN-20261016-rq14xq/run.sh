#!/usr/bin/env bash
# RUN-20261016-rq14xq: interactive claude under the PTY driver; a 40-line poem prompt, Escape
# 12 s into the turn, Ctrl-U to clear the restored prompt, then "Reply with just the word pelican."
# Written on 2026-11-02 from the run's notes and the captured driver log, not run at the time.
# From the repo root (claude must be on PATH and logged in; the repo must be a trusted
# workspace, since the trust dialog cannot be driven through the PTY):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14xq/run.sh
# Writes events.jsonl, server.log and pty.log(.txt) to out/ beside this script; the session
# runs in work/ beside it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike="$PWD/spikes/01-hooks"
out="$here/out"; work="$here/work"
rm -rf "$out" "$work"; mkdir -p "$out" "$work"

python3 "$spike/hook_server.py" "$out" 7796 5 > "$out/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
sleep 1

cd "$work"
python3 "$spike/pty_driver.py" "$out/pty.log" "$(cat <<JSON
[{"wait": "shortcuts", "timeout": 30}, {"sleep": 1},
 {"send": "Write a poem of exactly 40 lines about rivers. Write it line by line, no tools."},
 {"sleep": 1}, {"send": "\r"},
 {"wait_file_contains": ["$out/events.jsonl", "UserPromptSubmit"], "timeout": 30},
 {"sleep": 12}, {"send": "\u001b"}, {"sleep": 3}, {"send": "\u0015"}, {"sleep": 2},
 {"send": "Reply with just the word pelican."}, {"sleep": 1}, {"send": "\r"},
 {"wait_file_contains": ["$out/events.jsonl", "\"Stop\""], "timeout": 60},
 {"sleep": 2}, {"send": "/exit"}, {"sleep": 1}, {"send": "\r"}, {"sleep": 3}]
JSON
)" -- claude --settings "$spike/settings-pty2.json" --permission-mode default
