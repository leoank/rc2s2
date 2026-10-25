#!/usr/bin/env bash
# RUN-20261025-9fytgj: plan step 1 of EXP-20261025-25f6vm. A scratch keychain item shaped like Claude Code's
# credential (service "Claude Code-credentials-rc2s2test", a made-up token), a fake usage
# endpoint on 127.0.0.1:7799 that answers with the shape saved in EXP-20261016-rq1402 only when
# the bearer is that token and the beta header is present, a scratch hub, and the node with
# --claude-usage pointed at both, started without a controlling terminal (setsid) as launchd
# would. After three polls: the hub's health rows with usage.five_hour_pct, the item's
# attributes before and after (unchanged), what the endpoint saw, and the node's log. The item
# is deleted at the end. Touches no real credential.
# From the repo root:
#   nix develop -c notes/runs/RUN-20261025-9fytgj/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cargo build --quiet -p rc2s2-node -p rc2s2-hub
bin="$PWD/target/debug"
tmp="$(mktemp -d)"
svc="Claude Code-credentials-rc2s2test"
cleanup() {
  kill $(jobs -p) 2>/dev/null || true
  security delete-generic-password -s "$svc" > /dev/null 2>&1 || true
}
trap cleanup EXIT

token="rc2s2-test-$(python3 -c 'import secrets; print(secrets.token_hex(8))')"
security delete-generic-password -s "$svc" > /dev/null 2>&1 || true
security add-generic-password -a "$USER" -s "$svc" -w "{\"claudeAiOauth\":{\"accessToken\":\"$token\",\"refreshToken\":\"r\",\"expiresAt\":4102444800000}}"
before="$(security find-generic-password -s "$svc" | grep -v 'mdat' )"

python3 -c 'import secrets; print("run-node " + secrets.token_urlsafe(16))' > "$tmp/node-tokens"
awk '{print $2}' "$tmp/node-tokens" > "$tmp/node-token"
echo "ctl x" > "$tmp/client-tokens"
"$bin/rc2s2-hub" --listen 127.0.0.1:7700 --db "$tmp/hub.db" --node-tokens "$tmp/node-tokens" \
  --client-tokens "$tmp/client-tokens" 2> "$tmp/hub.err" &
sleep 1
EXPECT_TOKEN="$token" python3 -I "$here/fake_usage.py" "$here/../../experiments/EXP-20261016-rq1402/run-usage-endpoint/usage-response-shape.txt" \
  > "$tmp/endpoint.log" 2>&1 &
sleep 1
# No controlling terminal, as under launchd: a keychain prompt could not be answered.
# (macOS has no setsid command; a new session through Python's os.setsid.)
python3 -I -c 'import os, sys; os.setsid(); os.execvp(sys.argv[1], sys.argv[1:])' "$bin/rc2s2" node serve --hook-addr 127.0.0.1:7788 --health-secs 3600 --registry-secs 3600 \
  --hub http://127.0.0.1:7700 --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$tmp/node.sock" --ignore-cwd '*' \
  --claude-usage --claude-usage-secs 4 --claude-usage-url http://127.0.0.1:7799/api/oauth/usage \
  --claude-usage-service "$svc" < /dev/null > "$tmp/node.out" 2> "$tmp/node.err" &
sleep 14
after="$(security find-generic-password -s "$svc" | grep -v 'mdat')"
{
  echo "polls recorded by the hub: $(sqlite3 "$tmp/hub.db" "select count(distinct time_ms) from health where metric = 'usage.five_hour_pct'")"
  echo "last sample metrics: $(sqlite3 "$tmp/hub.db" "select group_concat(metric || '=' || value, ' ') from health where metric like 'usage.%' and time_ms = (select max(time_ms) from health where metric = 'usage.five_hour_pct')")"
  echo "endpoint: $(grep -c 'ok bearer matched, beta header present' "$tmp/endpoint.log") good requests, $(grep -c 'refused' "$tmp/endpoint.log") refused"
  echo "keychain item unchanged: $([ "$before" = "$after" ] && echo yes || echo no)"
  echo "node log: $(grep -c 'usage poll failed' "$tmp/node.err") failed polls; $(grep -o 'prompt may be waiting' "$tmp/node.err" | head -1)"
  echo "token in any log: $(grep -l "$token" "$tmp/node.err" "$tmp/node.out" "$tmp/hub.err" 2>/dev/null | wc -l | tr -d ' ') files"
} | tee "$here/summary.txt"
