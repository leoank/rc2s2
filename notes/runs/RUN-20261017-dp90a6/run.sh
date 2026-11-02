#!/usr/bin/env bash
# RUN-20261017-dp90a6: delivering a prompt to a running interactive session over its
# per-session messaging socket, in a default-mode and a bypass-mode receiver.
# Written on 2026-11-02 from the run's notes, not run at the time.
# Spawns throwaway `claude --model sonnet` sessions under spikes/01-hooks/pty_driver.py with
# cwd spikes/10-messaging-socket/work/ (deleted at the end), finds each one's pid in
# ~/.claude/sessions by that cwd, and sends frames with the spike's daemon.py, sink.py and
# probe.py. Only sessions this script started are targeted. daemon.py and probe.py read the
# throwaway session's peer token from its key file to send the auth line; they never print
# it. Outputs land in out/ beside this script.
# The static read of the Claude Code bundle (PROTOCOL.md beside the spike) is by hand.
# Needs the host's `claude` on PATH and a logged-in account (a few small sonnet turns).
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261017-dp90a6/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spikes="$PWD/spikes"
msg="$spikes/10-messaging-socket"
out="$here/out"
work="$msg/work"
mkdir -p "$out" "$work"
trap 'rm -rf "$work"' EXIT
cd "$work"

# The pid of the live interactive session whose registry entry has cwd = work/.
receiver_pid() {
  for _ in $(seq 60); do
    pid="$(python3 - "$work" <<'EOF'
import glob, json, os, sys
for p in glob.glob(os.path.expanduser("~/.claude/sessions/*.json")):
    e = json.load(open(p))
    if e.get("cwd") == sys.argv[1] and e.get("kind") == "interactive" and e.get("name") not in ("rc2s2-sink", "rc2s2-probe"):
        try:
            os.kill(e["pid"], 0); print(e["pid"]); break
        except OSError:
            pass
EOF
)"
    [ -n "$pid" ] && { echo "$pid"; return; }
    sleep 1
  done
  echo "no receiver registered" >&2; return 1
}

# 1. Default (prompting) receiver: a user frame without auth, then with auth.
python3 "$spikes/01-hooks/pty_driver.py" "$out/recv-default.pty" \
  '[{"wait": "shortcuts", "timeout": 30}, {"sleep": 95}, {"send": "/exit\r"}, {"sleep": 3}]' \
  -- claude --model sonnet &
driver=$!
pid="$(receiver_pid)"
sleep 5
python3 "$msg/daemon.py" --pid "$pid" --no-auth --text "Reply with exactly the word pong and nothing else." \
  | tee "$out/daemon-default.txt"
sleep 20
python3 "$msg/daemon.py" --pid "$pid" --text "Reply with exactly the word pong and nothing else." \
  | tee -a "$out/daemon-default.txt"
wait "$driver"
grep -a -E 'Another Claude session sent a message|pong' "$out/recv-default.pty.txt" || true

# 2. Bypass receiver: accept the one-time warning over the PTY; the inject is held.
#    A separate-process sink is the reply address (it gets connections but no bytes),
#    then the same-pid probe catches the held and expired receipts.
python3 "$spikes/01-hooks/pty_driver.py" "$out/recv-bypass.pty" \
  '[{"wait": "Yes, I accept", "timeout": 30}, {"send": "\u001b[B"}, {"sleep": 1}, {"send": "\r"},
    {"wait": "shortcuts", "timeout": 60}, {"sleep": 110}, {"send": "/exit\r"}, {"sleep": 3}]' \
  -- claude --model sonnet --dangerously-skip-permissions &
driver=$!
pid="$(receiver_pid)"
sleep 5
python3 "$msg/sink.py" "$out/sink.log" --seconds 200 &
sink=$!
sleep 2
python3 "$msg/daemon.py" --pid "$pid" --text "Reply with exactly the word pong." \
  --reply "uds:/tmp/cc-socks/$sink.sock" | tee "$out/daemon-bypass.txt"
python3 "$msg/daemon.py" --pid "$pid" --text "idle-subscription probe" --notify \
  --reply "uds:/tmp/cc-socks/$sink.sock" | tee -a "$out/daemon-bypass.txt"
# by hand: watch the approval dialog in out/recv-bypass.pty.txt and the OS notification;
# do not approve, so the held messages expire when the session exits.
python3 "$msg/probe.py" --pid "$pid" --text "probe held receipt test" --notify --seconds 150 \
  | tee "$out/probe-session.txt"
wait "$driver"
kill "$sink" 2>/dev/null || true
wait "$sink" || true
grep -a 'Held peer message' "$out/recv-bypass.pty.txt" || true
