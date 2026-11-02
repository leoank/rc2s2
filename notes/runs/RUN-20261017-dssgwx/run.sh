#!/usr/bin/env bash
# RUN-20261017-dssgwx: hooks over an AF_UNIX socket from inside the mxc seatbelt sandbox while
# loopback TCP is denied. Plain-shell probes (TCP refused, unix socket by grant and egress), then
# two `claude -p` turns whose Stop and PermissionRequest command hooks curl the host's
# unix_hook_server.py, which holds the permission 2 s and allows it. Uses the policy and
# settings files in experiments/EXP-20261016-rq1407/run-unix-hook/ (their paths are fixed at
# /private/tmp/rc2s2-sbx-unixhook, short enough for the 104-byte sun_path limit); logs and
# events.jsonl go to out/ beside this script.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# The run leased the owner's keychain credential. This script signs a scratch config dir in with
# `claude auth login` (a fresh grant, browser sign-in by hand), leases that grant into the
# sandbox's config dir as .credentials.json (never displayed), and logs it out at the end.
# From the repo root, in the spike's dev shell (macOS, `claude` on PATH, port 18765 free):
#   nix develop ./spikes/07-mxc -c \
#     notes/runs/RUN-20261017-dssgwx/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/07-mxc
exp=notes/experiments/EXP-20261016-rq1407/run-unix-hook
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT

cargo build --quiet --manifest-path "$spike/Cargo.toml"
m="$PWD/$spike/target/debug/mxc-spike"
bindir="$(dirname "$(readlink -f "$(command -v claude)")")"
out="$here/out"
mkdir -p "$out"
s=/private/tmp/rc2s2-sbx-unixhook
login="$(mktemp -d /private/tmp/rc2s2-login.XXXXXX)"
mkdir "$s"   # fails if a previous run left it behind
mkdir -p "$s"/{config,tmp,work,hooks,home}
svc() { printf 'Claude Code-credentials-%s' "$(printf %s "$1" | shasum -a 256 | cut -c1-8)"; }
hook= tcp=
cleanup() {
  kill "$hook" "$tcp" 2>/dev/null || true
  CLAUDE_CONFIG_DIR="$login" claude auth logout || true
  for d in "$login" "$s/config"; do security delete-generic-password -s "$(svc "$d")" >/dev/null 2>&1 || true; done
  rm -rf "$s" "$login"
}
trap cleanup EXIT

# by hand: complete the browser sign-in; the grant lands in the keychain item for $login
CLAUDE_CONFIG_DIR="$login" claude auth login

# host side: the unix-socket hook sink (hold 2 s) and a TCP listener as the negative control
python3 "$spike/unix_hook_server.py" "$out" "$s/hooks/hook.sock" 2 > "$out/hook-server.log" 2>&1 &
hook=$!
python3 -m http.server 18765 --bind 127.0.0.1 --directory "$s/work" 2> "$out/http.log" &
tcp=$!
sleep 1

probe() {  # probe <label> <policy> <cmd...>
  echo "== $1"
  "$m" "$exp/$2" -- "${@:3}" 2>&1 || true
}
post() { printf '{"hook_event_name":"%s"}' "$1"; }
uc=(/usr/bin/curl -sS --max-time 5 --unix-socket "$s/hooks/hook.sock" -X POST -H 'Content-Type: application/json')
{
  probe "1a: egress allow, hostLoopback deny, hooks rw: TCP 127.0.0.1:18765" policy-sh-unix-hook.json /usr/bin/curl -sS --max-time 5 http://127.0.0.1:18765/
  probe "1b: same: TCP localhost:18765" policy-sh-unix-hook.json /usr/bin/curl -sS --max-time 5 http://localhost:18765/
  probe "1c: same: POST over AF_UNIX" policy-sh-unix-hook.json "${uc[@]}" --data-binary "$(post ShTest1c)" http://hook/hook
  probe "1d: same: 3-line JSON piped through sh -c, --data-binary @-" policy-sh-unix-hook.json /bin/sh -c \
    "printf '{\"hook_event_name\":\"ShTest1d\",\n \"line2\":\"x\",\n \"line3\":\"y\"}\n' | /usr/bin/curl -sS --max-time 5 --unix-socket $s/hooks/hook.sock -X POST -H 'Content-Type: application/json' --data-binary @- http://hook/hook"
  probe "2: egress deny, hooks rw: POST over AF_UNIX" policy-sh-unix-hook-egress-deny.json "${uc[@]}" --data-binary "$(post ShTest2)" http://hook/hook
  probe "2b: egress deny: TCP 127.0.0.1:18765" policy-sh-unix-hook-egress-deny.json /usr/bin/curl -sS --max-time 5 http://127.0.0.1:18765/
  probe "3: hooks read-only: POST over AF_UNIX" policy-sh-unix-hook-readonly.json "${uc[@]}" --data-binary "$(post ShTest3)" http://hook/hook
  probe "4: hooks not granted, egress allow: POST over AF_UNIX" policy-sh-no-grant.json "${uc[@]}" --data-binary "$(post ShTest4)" http://hook/hook
  probe "4b: hooks not granted, egress deny: POST over AF_UNIX" policy-sh-no-grant-egress-deny.json "${uc[@]}" --data-binary "$(post ShTest4b)" http://hook/hook
  probe "4c: hooks in denied, egress allow: POST over AF_UNIX" policy-sh-denied.json "${uc[@]}" --data-binary "$(post ShTest4c)" http://hook/hook
} 2>&1 | tee "$out/test-sh-unix-hook.log"

# claude: lease the scratch grant, settings with both command hooks, policy with this claude's PATH
(umask 077; security find-generic-password -s "$(svc "$login")" -w > "$s/config/.credentials.json")
cp "$exp/settings-unix-hook.json" "$s/config/settings.json"
sed "s|/nix/store/[^\"]*-claude-code/bin|$bindir|" "$exp/policy-claude-unix-hook.json" > "$s/policy-claude.json"

{ echo "== claude run 1: Stop hook over AF_UNIX, hostLoopback deny"; date +%T
  ( time "$m" "$s/policy-claude.json" -- claude -p "Reply with just ok." --model sonnet --output-format json ) 2>&1 || true
  date +%T; } | tee "$out/test-claude-1-stop.log"
{ echo "== claude run 2: permission round trip, PermissionRequest held 2 s then allow"; date +%T
  ( time "$m" "$s/policy-claude.json" -- claude -p "Create a file named hello.txt containing hi in the current directory. Do not ask questions." \
      --permission-mode default --model sonnet --output-format json ) 2>&1 || true
  date +%T
  echo "work/hello.txt: $(cat "$s/work/hello.txt" 2>/dev/null || echo MISSING)"; } | tee "$out/test-claude-2-permission.log"
find "$s/config" -name '*.jsonl' -exec ls -l {} + | tee -a "$out/test-claude-2-permission.log"
