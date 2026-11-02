#!/usr/bin/env bash
# RUN-20261017-e4b03e: two AskUserQuestion questions in one call, answered by one answers map from the PermissionRequest hook.
# Written on 2026-11-02 from the run's notes, not run at the time. Starts hook_server_ask.py in
# `perm` mode on port 7792 with answers {"Which toppings?": ["Olives", "Onions"], "Pick a colour":
# "Red"}, drives `claude --model sonnet --permission-mode default --settings settings-ask-perm.json`
# under pty_driver.py with the two-question prompt, waits for Stop, then /exit. Events, server
# log and terminal log go to run-twoq/ beside this script.
# From the repo root, with a logged-in `claude` on PATH:
#   nix develop . -c \
#     notes/runs/RUN-20261017-e4b03e/run.sh
# The PTY driver does not answer the folder-trust dialog: the first time, run `claude` by hand
# in work/ beside this script and accept it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
hooks="$(pwd)/spikes/01-hooks"
prompt='Call the AskUserQuestion tool exactly once with TWO questions in the same call. Question 1: exact text "Which toppings?" header "Toppings" multiSelect true, options Cheese (dairy), Olives (briny), Peppers (spicy), Onions (sharp). Question 2: exact text "Pick a colour" header "Colour" multiSelect false, options Red (warm), Green (cool). After the tool returns, reply with ONLY the raw answers the tool reported, verbatim, nothing else. Do not call the tool a second time.'
out="$here/run-twoq"
work="$here/work"
rm -rf "$out"; mkdir -p "$out" "$work"
echo perm > "$out/ask_mode"
echo '{"Which toppings?": ["Olives", "Onions"], "Pick a colour": "Red"}' > "$out/ask_answers.json"

python3 "$hooks/hook_server_ask.py" "$out" 7792 > "$out/server.log" 2>&1 &
srv=$!
trap 'kill $srv 2>/dev/null' EXIT
sleep 1

steps="$(python3 -c 'import json, sys; p, ev = sys.argv[1:]; print(json.dumps([
  {"wait": "shortcuts", "timeout": 30}, {"send": p + "\n"}, {"sleep": 1}, {"send": "\r"},
  {"wait_file_contains": [ev, "\"Stop\""], "timeout": 120}, {"sleep": 3},
  {"send": "/exit"}, {"sleep": 1}, {"send": "\r"}, {"sleep": 2}]))' "$prompt" "$out/events.jsonl")"
(cd "$work" && python3 "$hooks/pty_driver.py" "$out/pty.log" "$steps" -- \
  claude --model sonnet --permission-mode default --settings "$hooks/settings-ask-perm.json")
echo "expected: PostToolUse answers \"Olives, Onions\" and \"Red\" in $out/events.jsonl and both lines in the terminal"
