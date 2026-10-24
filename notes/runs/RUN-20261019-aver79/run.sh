#!/usr/bin/env bash
# RUN-20261019-aver79: RUN-20261019-72r8bn again after the hub ignores re-read transcript lines
# (line_id, EXP-20261019-ayya0c). The hub and the node daemon end to end on this machine.
# Starts `rc2s2-hub` on a spare port with a scratch database and a generated token, and
# `rc2s2 node serve --hub` against the live machine state (read only, as RUN-20261017-rv3rjk).
# Posts marked synthetic hooks before, during and after a hub outage, restarts the node, then
# summarises the database with check.py into check.txt beside this script. The database
# holds transcript content of the person's sessions and the token, so both stay in a
# temporary directory that is removed at the end; only counts are written.
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-aver79/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=code/rc2s2-a-remote-command-and-control-session-server
hub_port="${RC2S2_HUB_PORT:-7701}"
hook_port="${RC2S2_PORT:-7788}"
per_phase=5

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$tmp"' EXIT

token="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
echo "run-node $token" > "$tmp/tokens"
echo "$token" > "$tmp/node-token"
unset token

start_hub() {
  "$bin/rc2s2-hub" --listen "127.0.0.1:$hub_port" --db "$tmp/hub.db" --node-tokens "$tmp/tokens" \
    2>> "$tmp/hub.err" &
  hub=$!
}
start_node() {
  "$bin/rc2s2" node serve --hook-addr "127.0.0.1:$hook_port" --health-secs 2 --registry-secs 1 \
    --hub "http://127.0.0.1:$hub_port" --token-file "$tmp/node-token" --name run-node \
    2>> "$tmp/node.err" &
  node=$!
}
post() { # phase: posts $per_phase marked hooks for a synthetic session
  for i in $(seq "$per_phase"); do
    curl -sf -o /dev/null -X POST -H 'Content-Type: application/json' \
      --data "{\"hook_event_name\":\"Notification\",\"session_id\":\"exp-0011\",\"cwd\":\"/tmp\",\"mark\":\"$1-$i\"}" \
      "http://127.0.0.1:$hook_port/hook"
  done
}
stop() { kill -INT "$1"; wait "$1" || true; }

start_hub
sleep 1
start_node
sleep 5
post before
sleep 2
stop "$hub"            # the hub goes away while the node keeps observing
post during
sleep 4
start_hub
sleep 8                # the node's backoff is at most a few seconds by now
post after
sleep 2
stop "$node"           # a fresh node process: new outbox, empty buffer
start_node
sleep 5
post restart
sleep 3
stop "$node"
stop "$hub"

{
  python3 "$here/check.py" "$tmp/hub.db" "$per_phase" "$(grep -c 'node registered' "$tmp/hub.err" || true)"
  # Warnings and errors, timestamps stripped: daemon messages only, never content or the token.
  for side in node hub; do
    echo "$side warnings and errors:"
    grep -E ' (WARN|ERROR) ' "$tmp/$side.err" | sed -E 's/^[^ ]+ +//' | sort | uniq -c || echo "  none"
  done
} | tee "$here/check.txt"
