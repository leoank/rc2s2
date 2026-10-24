#!/usr/bin/env bash
# RUN-20261019-c958hw: held permission requests resolved from a client against real Claude Code.
# Starts a scratch `rc2s2-hub` and `rc2s2 node serve --hub` (live state read only, hold of
# RC2S2_HOLD seconds), then runs three `claude -p` turns on haiku in a scratch directory,
# each asked to `touch` one file, with hooks pointed at the node through `--settings` from
# `rc2s2 node hooks` (the person's ~/.claude/settings.json is left alone). The files are
# outside Claude's working directory, so each `touch` needs approval. A watcher allows
# the first turn's request with `rc2s2 ctl`, denies the second with a message and leaves the
# third alone. check.py writes check.txt beside this script. Uses the host's `claude` and the
# person's login and costs three small model turns; `CLAUDE*` and `AI_AGENT` are stripped.
# Tokens and the database stay in a temporary directory that is removed.
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-c958hw/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=code/rc2s2-a-remote-command-and-control-session-server
hub_port="${RC2S2_HUB_PORT:-7701}"
hook_port="${RC2S2_PORT:-7788}"
hold="${RC2S2_HOLD:-8}"
claude="${CLAUDE_BIN:-$(command -v claude)}"
deny_message="Denied from rc2s2 ctl: use a file under /tmp instead."

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$tmp"' EXIT
mkdir "$tmp/work" "$tmp/out"

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
  --hold-secs "$hold" 2> "$tmp/node.err" &
"$bin/rc2s2" node hooks --hook-addr "127.0.0.1:$hook_port" --timeout 60 > "$tmp/settings.json"
sleep 2

ctl() { "$bin/rc2s2" ctl --hub "http://127.0.0.1:$hub_port" --token-file "$tmp/client-token" "$@"; }
# Waits for the turn's request to be open and resolves it: allow, deny or nothing.
resolver() {
  local id="" tries=300
  while [ -z "$id" ]; do
    id="$(ctl pending | awk '$5 == "Bash" { print $2; exit }')"
    tries=$((tries - 1))
    if [ "$tries" = 0 ]; then echo "no request within 60 s"; return; fi
    sleep 0.2
  done
  case "$1" in
    allow) ctl allow run-node "$id" ;;
    deny) ctl deny run-node "$id" --message "$deny_message" ;;
  esac
}

unset_args=()
while IFS='=' read -r name _; do
  unset_args+=(-u "$name")
done < <(env | grep -E '^(CLAUDE[^=]*|AI_AGENT)=' || true)
turn() { # decision file
  resolver "$1" > "$tmp/$1.ctl" 2>&1 &
  local watcher=$!
  (cd "$tmp/work" && env "${unset_args[@]}" "$claude" -p \
    "Run exactly this one shell command and nothing else: touch $tmp/out/$2 -- then reply in one line saying whether it ran, quoting any message you were given about it." \
    --settings "$tmp/settings.json" --model haiku) > "$tmp/$1.reply" 2>&1 || true
  wait "$watcher"
}
turn allow allowed.txt
turn deny denied.txt
turn ignore ignored.txt
sleep 1

python3 -I "$here/check.py" "$tmp/hub.db" "$tmp/out" "$tmp" "$deny_message" | tee "$here/check.txt"
