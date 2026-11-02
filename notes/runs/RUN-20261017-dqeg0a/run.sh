#!/usr/bin/env bash
# RUN-20261017-dqeg0a: how Claude Code refreshes ~/.claude.json cachedUsageUtilization.
# Written on 2026-11-02 from the run's notes, not run at the time. Greps the printable strings
# of the installed claude binary for the usage code, then samples ~/.claude.json (mtime and
# fetchedAtMs only) while one interactive session (start, /usage, Esc, 70 s idle, one prompt,
# /exit) and two headless runs (a prompt, then /usage) go by. Outputs go to out/ beside this
# script; the headless stream, which carries session ids, stays in a temporary directory.
# From the repo root, with a logged-in `claude` on PATH:
#   nix develop . -c \
#     notes/runs/RUN-20261017-dqeg0a/run.sh
# The PTY driver does not answer the folder-trust dialog: the first time, run `claude` by hand
# in work/ beside this script and accept it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
hooks=spikes/01-hooks
sampler=notes/experiments/EXP-20261016-rq1402/run-usage-refresh/sampler.py
out="$here/out"
work="$here/work"
rm -rf "$out"
mkdir -p "$out" "$work"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Static read: the usage endpoint, cache and get_usage strings (minified names change per release).
bin="$(readlink -f "$(command -v claude)")"
[ -e "$(dirname "$bin")/.claude-wrapped" ] && bin="$(dirname "$bin")/.claude-wrapped"
strings "$bin" | grep -o -E '.{0,200}(/api/oauth/usage|cachedUsageUtilization|get_usage).{0,200}' \
  > "$out/strings-usage.txt" || true

python3 "$sampler" 300 "$out/claude-json-sampler.log" &
smp=$!

steps="$(python3 -c 'import json; print(json.dumps([
  {"wait": "\\? for shortcuts|bypass permissions|Try \"|Welcome to Claude", "timeout": 30},
  {"sleep": 6}, {"send": "/usage"}, {"sleep": 2}, {"send": "\r"},
  {"wait": "Current session|Current week|Failed to load|usage data|Loading", "timeout": 30},
  {"sleep": 10}, {"send": "\u001b"}, {"sleep": 3}, {"sleep": 70},
  {"send": "Reply with ok"}, {"sleep": 1}, {"send": "\r"},
  {"wait": "[⏺●]\\s*[Oo][Kk]", "timeout": 60},
  {"sleep": 10}, {"send": "/exit"}, {"sleep": 2}, {"send": "\r"}, {"sleep": 6}]))')"
root="$(pwd)"
(cd "$work" && python3 "$root/$hooks/pty_driver.py" "$tmp/pty-interactive" "$steps" -- \
  claude --model sonnet --permission-mode default)
cp "$tmp/pty-interactive.txt" "$out/pty-interactive.txt"

(cd "$work" && claude -p "Reply with ok" --output-format stream-json --verbose --model sonnet \
  < /dev/null > "$tmp/p-prompt.stream.jsonl")
python3 -c 'import json, sys
for l in open(sys.argv[1]):
    o = json.loads(l)
    if o.get("type") == "rate_limit_event":
        print(json.dumps(o.get("rate_limit_info")))' "$tmp/p-prompt.stream.jsonl" > "$out/rate-limit-event.json"
(cd "$work" && claude -p "/usage" --model sonnet < /dev/null > "$out/p-usage.txt")

sleep 4
kill "$smp" 2>/dev/null || true
echo "sampler log, strings and /usage text in $out"
