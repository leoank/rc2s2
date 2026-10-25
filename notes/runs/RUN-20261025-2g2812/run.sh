#!/usr/bin/env bash
# RUN-20261025-2g2812: turn durations and token totals against /cost (ADR-0030) in a real interactive
# Claude Code session with a tool call, an interrupted turn and a subagent. A scratch
# `rc2s2-hub` on 127.0.0.1:7700 and `rc2s2 node serve --hub` (live state read only, hooks through
# `--settings`), then cost_check.py starts haiku sessions under `rc2s2 run` in a scratch
# directory under the project's `target/live-work-*` (ignored by the owner's live node,
# ADR-0028), drives the terminal through a PTY and the client side through `rc2s2 ctl`, and
# writes summary.txt beside this script from the hub's record of those sessions only.
# Uses the host's `claude` and the person's login (about six small turns and one subagent).
# From the repo root:
#   nix develop -c notes/runs/RUN-20261025-2g2812/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=.
claude="${CLAUDE_BIN:-$(command -v claude)}"

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
sock="$(mktemp -d /tmp/rc2s2w.XXXX)"
work="$PWD/$project/target/live-work-$$"
cleanup() {
  kill $(jobs -p) 2>/dev/null || true
  rm -rf "$work" "$sock"
}
trap cleanup EXIT
mkdir -p "$work"

token() { python3 -c 'import secrets; print(secrets.token_urlsafe(32))'; }
node_token="$(token)" client_token="$(token)"
echo "run-node $node_token" > "$tmp/node-tokens"
echo "ctl $client_token" > "$tmp/client-tokens"
echo "$node_token" > "$tmp/node-token"
echo "$client_token" > "$tmp/client-token"
unset node_token client_token

"$bin/rc2s2-hub" --listen 127.0.0.1:7700 --db "$tmp/hub.db" \
  --node-tokens "$tmp/node-tokens" --client-tokens "$tmp/client-tokens" 2> "$tmp/hub.err" &
sleep 1
"$bin/rc2s2" node serve --hook-addr 127.0.0.1:7788 --health-secs 60 --registry-secs 1 \
  --hub http://127.0.0.1:7700 --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$sock/node.sock" 2> "$tmp/node.err" &
"$bin/rc2s2" node hooks --hook-addr 127.0.0.1:7788 --timeout 600 > "$tmp/settings.json"
sleep 1

python3 -I "$here/cost_check.py" --bin "$bin" --socket "$sock/node.sock" --settings "$tmp/settings.json" \
  --work "$work" --db "$tmp/hub.db" --claude "$claude" --log "$tmp/pty.log" \
  --token-file "$tmp/client-token" | tee "$here/summary.txt"
