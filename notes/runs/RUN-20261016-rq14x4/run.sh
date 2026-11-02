#!/usr/bin/env bash
# RUN-20261016-rq14x4: an interactive session whose AskUserQuestion is answered by the PermissionRequest hook.
# Written on 2026-11-02 from the run's notes, not run at the time. Starts hook_server.py in
# `perm` mode (port 7792, hold 5 s; it answers with the second option, Coffee), drives
# `claude --settings settings-ask-perm.json` under pty_driver.py with the drink prompt, waits
# for Stop, then /exit. Events, server log and terminal log go to run-perm-interactive/ beside
# this script.
# From the repo root, with a logged-in `claude` on PATH:
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14x4/run.sh
# The PTY driver does not answer the folder-trust dialog: the first time, run `claude` by hand
# in work/ beside this script and accept it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
hooks="$(pwd)/spikes/01-hooks"
prompt='Use the AskUserQuestion tool to ask me one question: which drink I prefer, with exactly two options labelled Tea and Coffee. After the tool returns, reply with exactly the label I chose and nothing else.'
out="$here/run-perm-interactive"
work="$here/work"
rm -rf "$out"; mkdir -p "$out" "$work"
echo perm > "$out/ask_mode"

python3 "$hooks/hook_server.py" "$out" 7792 5 > "$out/server.log" 2>&1 &
srv=$!
trap 'kill $srv 2>/dev/null' EXIT
sleep 1

steps="$(python3 -c 'import json, sys; p, ev = sys.argv[1:]; print(json.dumps([
  {"wait": "shortcuts", "timeout": 30}, {"sleep": 1}, {"send": p}, {"sleep": 1}, {"send": "\r"},
  {"wait_file_contains": [ev, "\"Stop\""], "timeout": 120}, {"sleep": 2},
  {"send": "/exit"}, {"sleep": 1}, {"send": "\r"}, {"sleep": 3}]))' "$prompt" "$out/events.jsonl")"
(cd "$work" && python3 "$hooks/pty_driver.py" "$out/pty.log" "$steps" -- \
  claude --settings "$hooks/settings-ask-perm.json")
echo "expected: the dialog drawn briefly, 'Allowed by PermissionRequest hook' and the reply Coffee in $out/pty.log.txt"
