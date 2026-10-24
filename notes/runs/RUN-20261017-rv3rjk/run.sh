#!/usr/bin/env bash
# RUN-20261017-rv3rjk: the node daemon against this machine's live state, read only.
# Runs `rc2s2 node serve` for a few seconds on a spare port, posts one synthetic hook, and
# writes a summary by type and kind to summary.txt beside this script. The raw output holds
# transcript content of the person's sessions, so it stays in a temporary directory.
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261017-rv3rjk/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=code/rc2s2-a-remote-command-and-control-session-server
port="${RC2S2_PORT:-7787}"
seconds="${RC2S2_SECONDS:-6}"

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node
bin="$project/target/debug/rc2s2"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

"$bin" node serve --hook-addr "127.0.0.1:$port" --health-secs 2 --registry-secs 1 \
  > "$tmp/out.jsonl" 2> "$tmp/err.log" &
daemon=$!
sleep 3
curl -s -o "$tmp/hook-reply" -w '%{http_code}' -X POST -H 'Content-Type: application/json' \
  --data '{"hook_event_name":"Notification","session_id":"smoke-test","cwd":"/tmp"}' \
  "http://127.0.0.1:$port/hook" > "$tmp/hook-status"
sleep "$((seconds - 3))"
kill -INT "$daemon"
wait "$daemon" || true

python3 "$here/summarize.py" "$tmp/out.jsonl" "$(cat "$tmp/hook-status")" "$(cat "$tmp/hook-reply")" \
  | tee "$here/summary.txt"
