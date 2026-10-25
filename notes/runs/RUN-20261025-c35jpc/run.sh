#!/usr/bin/env bash
# RUN-20261025-c35jpc: plan step 2 of EXP-20261025-25f6vm. The node's --claude-usage poller
# against the real keychain item "Claude Code-credentials" and the real usage endpoint,
# started without a controlling terminal as launchd would, reporting to a scratch hub.
# Read only: the item is never written. Prints no token, only an 8-hex hash of it.
# Afterwards: the hub's live usage rows, the item's attributes before and after, the token's
# hash and expiry before and after, `claude auth status`, and `claude -p /usage` to compare.
# RUN-20261025-c5xjbh (step 3) reuses this script with a longer DURATION_S across a refresh.
# From the repo root:
#   nix develop -c notes/runs/RUN-20261025-c35jpc/run.sh
# Env: DURATION_S (default 200), POLL_SECS (default 60), OUT (default this run's directory).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${OUT:-$here}"
duration="${DURATION_S:-200}"
poll="${POLL_SECS:-60}"
cargo build --quiet -p rc2s2-node -p rc2s2-hub
bin="$PWD/target/debug"
tmp="$(mktemp -d)"
svc="Claude Code-credentials"
trap 'kill $(jobs -p) 2>/dev/null || true' EXIT

# The item's attributes (no secret) and the token's hash and expiry, never the token.
attrs() { /usr/bin/security find-generic-password -s "$svc" | grep -v '0x00000007\|"gena"'; }
cred() {
  /usr/bin/security find-generic-password -s "$svc" -w | python3 -I -c '
import sys, json, hashlib, datetime
c = json.load(sys.stdin)["claudeAiOauth"]
exp = datetime.datetime.fromtimestamp(c["expiresAt"] / 1000, datetime.timezone.utc)
print("token", hashlib.sha256(c["accessToken"].encode()).hexdigest()[:8], "expires", exp.strftime("%H:%M:%SZ"))'
}
before_attrs="$(attrs)"
before_cred="$(cred)"

python3 -c 'import secrets; print("run-node " + secrets.token_urlsafe(16))' > "$tmp/node-tokens"
awk '{print $2}' "$tmp/node-tokens" > "$tmp/node-token"
echo "ctl x" > "$tmp/client-tokens"
"$bin/rc2s2-hub" --listen 127.0.0.1:7700 --db "$tmp/hub.db" --node-tokens "$tmp/node-tokens" \
  --client-tokens "$tmp/client-tokens" 2> "$tmp/hub.err" &
sleep 1
# No controlling terminal, as under launchd (macOS has no setsid command).
python3 -I -c 'import os, sys; os.setsid(); os.execvp(sys.argv[1], sys.argv[1:])' "$bin/rc2s2" node serve \
  --hook-addr 127.0.0.1:7788 --health-secs 3600 --registry-secs 3600 \
  --hub http://127.0.0.1:7700 --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$tmp/node.sock" --ignore-cwd '*' \
  --claude-usage --claude-usage-secs "$poll" < /dev/null > "$tmp/node.out" 2> "$tmp/node.err" &

# Watch the item's modification time once a minute, to see a refresh when one happens.
end=$(( $(date +%s) + duration ))
: > "$out/mdat.txt"
while [ "$(date +%s)" -lt "$end" ]; do
  echo "$(date -u +%H:%M:%SZ) $(attrs | grep mdat | sed 's/.*"\(.*\)\\000"/\1/')" >> "$out/mdat.txt"
  sleep 60
done
after_attrs="$(attrs)"
after_cred="$(cred)"

live="time_ms in (select time_ms from health where metric = 'usage.age_s' and value = 0)"
{
  echo "polls: every ${poll}s for ${duration}s"
  echo "live polls recorded by the hub: $(sqlite3 "$tmp/hub.db" "select count(distinct time_ms) from health where metric = 'usage.five_hour_pct' and $live")"
  echo "live samples (UTC time, 5-hour %, 7-day %):"
  sqlite3 "$tmp/hub.db" "select strftime('%H:%M:%S', time_ms / 1000, 'unixepoch'),
      max(case when metric = 'usage.five_hour_pct' then value end),
      max(case when metric = 'usage.seven_day_pct' then value end)
    from health where $live group by time_ms order by time_ms" | sed 's/^/  /'
  echo "failed polls: $(grep -c 'usage poll failed' "$tmp/node.err" || true)"
  grep 'usage poll failed' "$tmp/node.err" | sed 's/^/  /' || true
  echo "credential before: $before_cred"
  echo "credential after:  $after_cred"
  echo "item attributes unchanged: $([ "$before_attrs" = "$after_attrs" ] && echo yes || echo "no, mdat $(echo "$after_attrs" | grep mdat | sed 's/.*"\(.*\)\\000"/\1/')")"
  echo "claude auth status loggedIn: $(claude auth status | python3 -I -c 'import sys, json; print(json.load(sys.stdin)["loggedIn"])')"
  echo "token in any log: $(/usr/bin/security find-generic-password -s "$svc" -w | python3 -I -c '
import sys, json
t = json.load(sys.stdin)["claudeAiOauth"]["accessToken"]
print(sum(t in open(f, errors="replace").read() for f in sys.argv[1:]), "files")' "$tmp/node.err" "$tmp/node.out" "$tmp/hub.err")"
  echo "claude -p /usage at the end:"
  (cd "$tmp" && claude -p "/usage" 2>&1 | grep -E '^Current' | sed 's/^/  /')
} | tee "$out/summary.txt"
