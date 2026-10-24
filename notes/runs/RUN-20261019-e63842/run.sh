#!/usr/bin/env bash
# RUN-20261019-e63842: intervention in one interactive Claude Code session under `rc2s2 run`.
# Starts a scratch `rc2s2-hub` and `rc2s2 node serve --hub` (live state read only, its own
# wrapper socket), then driver.py starts one interactive haiku session under `rc2s2 run`
# through a PTY, in a scratch directory under the project's `target/` (inside the repo, so
# the workspace is already trusted; the trust dialog cannot be driven), with hooks from
# `rc2s2 node hooks` through `--settings`. The driver plays the person at the terminal and
# uses `rc2s2 ctl` for the client: a question answered from the client, a command answered
# by Enter at the terminal, a prompt over the messaging socket, an interrupt with a
# follow-up, and a kill. It writes summary.txt beside this script from the hub's record of
# this session only. Uses the host's `claude` and the person's login (about six small
# turns); tokens, the database and the terminal output stay in a temporary directory.
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-e63842/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=code/rc2s2-a-remote-command-and-control-session-server
hub_port="${RC2S2_HUB_PORT:-7701}"
hook_port="${RC2S2_PORT:-7788}"
claude="${CLAUDE_BIN:-$(command -v claude)}"

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
work="$PWD/$project/target/live-work-$$"
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$tmp" "$work"' EXIT
mkdir -p "$work" "$tmp/out" "$tmp/w"

token() { python3 -c 'import secrets; print(secrets.token_urlsafe(32))'; }
node_token="$(token)" client_token="$(token)"
echo "run-node $node_token" > "$tmp/node-tokens"
echo "run-ctl $client_token" > "$tmp/client-tokens"
echo "$node_token" > "$tmp/node-token"
echo "$client_token" > "$tmp/client-token"
unset node_token client_token

"$bin/rc2s2-hub" --listen "127.0.0.1:$hub_port" --db "$tmp/hub.db" \
  --node-tokens "$tmp/node-tokens" --client-tokens "$tmp/client-tokens" 2> "$tmp/hub.err" &
sleep 1
"$bin/rc2s2" node serve --hook-addr "127.0.0.1:$hook_port" --health-secs 60 --registry-secs 1 \
  --hub "http://127.0.0.1:$hub_port" --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$tmp/w/node.sock" 2> "$tmp/node.err" &
"$bin/rc2s2" node hooks --hook-addr "127.0.0.1:$hook_port" --timeout 600 > "$tmp/settings.json"
sleep 2

python3 -I "$here/driver.py" --bin "$bin" --hub "http://127.0.0.1:$hub_port" \
  --client-token "$tmp/client-token" --socket "$tmp/w/node.sock" --settings "$tmp/settings.json" \
  --work "$work" --out "$tmp/out" --db "$tmp/hub.db" --claude "$claude" --log "$tmp/pty.log" \
  | tee "$here/summary.txt"
