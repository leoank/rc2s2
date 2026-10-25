#!/usr/bin/env bash
# RUN-20261025-166zqd: the rebuilt Android app (phase 1, 2026-10-25) on the emulator intervening
# in one real interactive Claude Code session under `rc2s2 run`, as RUN-20261019-mvz0r3 did with
# the first app. Starts a scratch `rc2s2-hub` on 127.0.0.1:7700 (the emulator reaches it as
# 10.0.2.2:7700) and `rc2s2 node serve --hub` (live state read only, hooks through `--settings`),
# boots the emulator unless one already answers on port 5584, installs the app fresh, then
# app_ui.py starts a haiku session under `rc2s2 run` in a scratch directory under the project's
# `target/` and drives both the terminal (through a PTY) and the app (through uiautomator and
# adb input): connect, answer a question, deny with a reason, prompt, interrupt with a
# follow-up, kill. It writes summary.txt beside this script from the hub's record of this
# session only; screenshots, tokens and the database stay in a temporary directory.
# Uses the host's `claude` and the person's login (about six small turns).
# From the repo root:
#   nix develop .#android -c notes/runs/RUN-20261025-166zqd/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
project=.
claude="${CLAUDE_BIN:-$(command -v claude)}"
dev=emulator-5584

cargo build --quiet --manifest-path "$project/Cargo.toml" -p rc2s2-node -p rc2s2-hub
(cd "$project/android" && gradle --no-daemon -q assembleDebug)
bin="$PWD/$project/target/debug"
tmp="$(mktemp -d)"
# The wrapper socket path must stay short (SUN_LEN): not under $tmp when that is deep.
sock="$(mktemp -d /tmp/rc2s2w.XXXX)"
work="$PWD/$project/target/live-work-$$"
booted=0
cleanup() {
  kill $(jobs -p) 2>/dev/null || true
  [ "$booted" = 1 ] && adb -s $dev emu kill > /dev/null 2>&1 || true
  rm -rf "$work" "$sock"
}
trap cleanup EXIT
mkdir -p "$work" "$tmp/out" "${RC2S2_KEEP:-$tmp}/shots"

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
"$bin/rc2s2" node serve --hook-addr 127.0.0.1:7788 --health-secs 20 --registry-secs 1 \
  --hub http://127.0.0.1:7700 --token-file "$tmp/node-token" --name run-node \
  --wrapper-socket "$sock/node.sock" 2> "$tmp/node.err" &
"$bin/rc2s2" node hooks --hook-addr 127.0.0.1:7788 --timeout 600 > "$tmp/settings.json"

if [ "$(adb -s $dev get-state 2>/dev/null || true)" != device ]; then
  emulator -avd rc2s2 -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect \
    -port 5584 > "$tmp/emulator.log" 2>&1 &
  booted=1
  adb -s $dev wait-for-device
  until [ "$(adb -s $dev shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; do sleep 2; done
fi
adb -s $dev install -r "$project/android/app/build/outputs/apk/debug/app-debug.apk" > /dev/null
adb -s $dev shell pm clear me.leoank.rc2s2 > /dev/null
# Granted up front: the permission dialog would cover the app the driver reads.
adb -s $dev shell pm grant me.leoank.rc2s2 android.permission.POST_NOTIFICATIONS

python3 -I "$here/app_ui.py" --bin "$bin" --socket "$sock/node.sock" --settings "$tmp/settings.json" \
  --work "$work" --out "$tmp/out" --db "$tmp/hub.db" --claude "$claude" --log "$tmp/pty.log" \
  --token-file "$tmp/client-token" --shots "${RC2S2_KEEP:-$tmp}/shots" --device $dev | tee "$here/summary.txt"
