#!/usr/bin/env bash
# RUN-20261017-q4m07f: hooks from a real Claude Code session reach the node daemon.
# Starts `rc2s2 node serve` on a spare port, runs one `claude -p` turn on haiku in a scratch
# directory with `--settings` from `rc2s2 node hooks` (the person's ~/.claude/settings.json
# is left alone), then checks the daemon's output with check.py into check.txt beside this
# script. Uses the host's `claude` and the person's login, like any extra terminal, and
# costs one small model turn. `CLAUDE*` and `AI_AGENT` are stripped so a session started
# from inside another one is not taken for its child.
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261017-q4m07f/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=.
port="${RC2S2_PORT:-7787}"
claude="${CLAUDE_BIN:-$(command -v claude)}"

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node
bin="$PWD/$project/target/debug/rc2s2"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/work"

"$bin" node hooks --hook-addr "127.0.0.1:$port" --timeout 30 > "$tmp/settings.json"
"$bin" node serve --hook-addr "127.0.0.1:$port" --registry-secs 1 --health-secs 60 \
  > "$tmp/daemon.jsonl" 2> "$tmp/daemon.err" &
daemon=$!
sleep 2

unset_args=()
while IFS='=' read -r name _; do
  unset_args+=(-u "$name")
done < <(env | grep -E '^(CLAUDE[^=]*|AI_AGENT)=' || true)
(cd "$tmp/work" && env "${unset_args[@]}" "$claude" -p \
  "Run this shell command and reply with its output only: echo rc2s2-hook-check" \
  --settings "$tmp/settings.json" --allowedTools "Bash(echo:*)" --model haiku) > "$tmp/claude.out"
sleep 3
kill -INT "$daemon"
wait "$daemon" || true

{ echo "claude replied: $(cat "$tmp/claude.out")"; python3 "$here/check.py" "$tmp/daemon.jsonl"; } \
  | tee "$here/check.txt"
