#!/usr/bin/env bash
# RUN-20261019-mvz0r3: the Android app on the emulator intervening in one real interactive
# Claude Code session under `rc2s2 run`. Starts a scratch `rc2s2-hub` on 127.0.0.1:7700 (the
# emulator reaches it as 10.0.2.2:7700) and `rc2s2 node serve --hub` (live state read only,
# hooks through `--settings`), boots the emulator with the app freshly installed, then
# app_live.py starts a haiku session under `rc2s2 run` in a scratch directory under the
# project's `target/` and drives both the terminal (through a PTY) and the app (through
# uiautomator and adb input). It writes summary.txt beside this script from the hub's record
# of this session only; screenshots, tokens and the database stay in a temporary directory.
# Uses the host's `claude` and the person's login (about five small turns).
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server#android -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-mvz0r3/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=code/rc2s2-a-remote-command-and-control-session-server
claude="${CLAUDE_BIN:-$(command -v claude)}"

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
(cd "$project/android" && gradle --no-daemon -q assembleDebug)
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
work="$PWD/$project/target/live-work-$$"
cleanup() {
  kill $(jobs -p) 2>/dev/null || true
  adb -s emulator-5584 emu kill > /dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
mkdir -p "$work" "$tmp/out" "$tmp/w" "${RC2S2_KEEP:-$tmp}/shots"

token() { python3 -c 'import secrets; print(secrets.token_urlsafe(32))'; }
node_token="$(token)" client_token="$(token)"
echo "run-node $node_token" > "$tmp/node-tokens"
echo "phone $client_token" > "$tmp/client-tokens"
echo "$node_token" > "$tmp/node-token"
echo "$client_token" > "$tmp/client-token"
unset node_token client_token

"$bin/rc2s2-hub" --listen 127.0.0.1:7700 --db "$tmp/hub.db" \
  --node-tokens "$tmp/node-tokens" --client-tokens "$tmp/client-tokens" 2> "$tmp/hub.err" &
sleep 1
"$bin/rc2s2" node serve --hook-addr 127.0.0.1:7788 --health-secs 60 --registry-secs 1 \
  --hub http://127.0.0.1:7700 --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$tmp/w/node.sock" 2> "$tmp/node.err" &
"$bin/rc2s2" node hooks --hook-addr 127.0.0.1:7788 --timeout 600 > "$tmp/settings.json"

emulator -avd rc2s2 -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect \
  -port 5584 > "$tmp/emulator.log" 2>&1 &
adb -s emulator-5584 wait-for-device
until [ "$(adb -s emulator-5584 shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; do sleep 2; done
adb -s emulator-5584 install -r "$project/android/app/build/outputs/apk/debug/app-debug.apk" > /dev/null
adb -s emulator-5584 shell pm clear me.leoank.rc2s2 > /dev/null
# Granted up front: the permission dialog would cover the app the driver reads.
adb -s emulator-5584 shell pm grant me.leoank.rc2s2 android.permission.POST_NOTIFICATIONS

python3 -I "$here/app_live.py" --bin "$bin" --socket "$tmp/w/node.sock" --settings "$tmp/settings.json" \
  --work "$work" --out "$tmp/out" --db "$tmp/hub.db" --claude "$claude" --log "$tmp/pty.log" \
  --token-file "$tmp/client-token" --shots "${RC2S2_KEEP:-$tmp}/shots" | tee "$here/summary.txt"
