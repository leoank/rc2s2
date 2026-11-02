#!/usr/bin/env bash
# RUN-20261016-rq148j: interactive claude under the PTY driver with the hook server holding
# PermissionRequest for 60 s while the terminal presses Enter on the visible dialog at ~27 s.
# Written on 2026-11-02 from the run's notes and the captured driver log, not run at the time.
# From the repo root (claude must be on PATH and logged in; the repo must be a trusted
# workspace, since the trust dialog cannot be driven through the PTY):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq148j/run.sh
# Writes events.jsonl, server.log and pty.log(.txt) to out/ beside this script; the session
# runs in work/ beside it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike="$PWD/spikes/01-hooks"
out="$here/out"; work="$here/work"
rm -rf "$out" "$work"; mkdir -p "$out" "$work"

python3 "$spike/hook_server.py" "$out" 7782 60 > "$out/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
sleep 1

cd "$work"
python3 "$spike/pty_driver.py" "$out/pty.log" "$(cat <<JSON
[{"wait": "shortcuts", "timeout": 30}, {"sleep": 1},
 {"send": "Please run this shell command with the Bash tool and then stop: date > now.txt"},
 {"sleep": 1}, {"send": "\r"},
 {"wait_file_contains": ["$out/events.jsonl", "PermissionRequest"], "timeout": 60},
 {"sleep": 25}, {"send": "\r"},
 {"wait_file_contains": ["$out/events.jsonl", "\"Stop\""], "timeout": 60},
 {"sleep": 2}, {"send": "/exit"}, {"sleep": 1}, {"send": "\r"}, {"sleep": 3}]
JSON
)" -- claude --settings "$spike/settings-race.json" --permission-mode default
