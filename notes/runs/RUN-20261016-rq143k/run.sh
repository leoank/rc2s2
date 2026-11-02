#!/usr/bin/env bash
# RUN-20261016-rq143k: test (c) of EXP-20261016-rq1407, `claude -p "reply with the single word ok"`
# under the mxc seatbelt backend, attempt by attempt as the notes number them (5 is left out: it
# is 6 without USER). Policies are generated into a scratch dir under /private/tmp; logs go to
# out/ beside this script; transcripts stay in the scratch dir, which is removed at the end.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# The run used the owner's own keychain credential. This script never reads or copies it: it
# signs a scratch config dir in with `claude auth login` (a fresh grant, browser sign-in by
# hand), points the keychain attempts at that grant and leases it as a file for the others, and
# logs the grant out at the end.
# From the repo root, in the spike's dev shell (macOS, `claude` on PATH, port 18765 free):
#   nix develop ./spikes/07-mxc -c \
#     notes/runs/RUN-20261016-rq143k/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/07-mxc
for v in $(env | grep -o '^CLAUDE[^=]*'); do unset "$v"; done
unset AI_AGENT

cargo build --quiet --manifest-path "$spike/Cargo.toml"
m="$PWD/$spike/target/debug/mxc-spike"
bindir="$(dirname "$(readlink -f "$(command -v claude)")")"
out="$here/out"
mkdir -p "$out"
s="$(mktemp -d /private/tmp/rc2s2-sbx.XXXXXX)"
mkdir -p "$s"/{config,tmp,work,home,login,www}
svc() { printf 'Claude Code-credentials-%s' "$(printf %s "$1" | shasum -a 256 | cut -c1-8)"; }
srv=
cleanup() {
  kill "$srv" 2>/dev/null || true
  rm -f "$s/config/.credentials.json"
  CLAUDE_CONFIG_DIR="$s/login" claude auth logout || true
  for d in login config; do security delete-generic-password -s "$(svc "$s/$d")" >/dev/null 2>&1 || true; done
  rm -rf "$s"
}
trap cleanup EXIT

# by hand: complete the browser sign-in; the grant lands in the keychain item for $s/login
CLAUDE_CONFIG_DIR="$s/login" claude auth login

# pol <n> <overrides>: policy v1 of attempt 1 with top-level keys and env entries overridden
# (null removes one); @S, @HOME, @USER expand; "readonly": "@CLOSURE" is the claude closure + work
pol() {
  python3 -c '
import json, os, sys
s, bindir, raw = sys.argv[1:4]
for k, v in (("@S", s), ("@HOME", os.environ["HOME"]), ("@USER", os.environ["USER"])):
    raw = raw.replace(k, v)
over = json.loads(raw)
p = {"readonly": ["/nix/store"], "readwrite": [s + "/config", s + "/work", s + "/tmp"],
     "egress": "allow", "ingress": "allow", "hostLoopback": "allow", "keychain": False,
     "env": {"PATH": bindir + ":/usr/bin:/bin", "HOME": s + "/home", "CLAUDE_CONFIG_DIR": s + "/config"},
     "cwd": s + "/work", "timeoutMs": 120000}
p["env"].update(over.pop("env", {}))
p.update(over)
if p["readonly"] == "@CLOSURE":
    p["readonly"] = open(s + "/closure").read().split() + [s + "/work"]
p = {k: v for k, v in p.items() if v is not None}
p["env"] = {k: v for k, v in p["env"].items() if v is not None}
print(json.dumps(p, indent=2))' "$s" "$bindir" "$2" > "$s/policy-$1.json"
}
attempt() {  # attempt <n> <label>
  log="$out/test-c-claude-$1.log"
  echo "== (c) attempt $1: $2" > "$log"
  ( time "$m" "$s/policy-$1.json" -- claude -p "reply with the single word ok" ) >> "$log" 2>&1 || true
  cat "$log"
}

tmpdir='"CLAUDE_CODE_TMPDIR": "@S/tmp"'
pol 1 '{}'
attempt 1 "policy v1, no scratch tmp (expect EPERM on /tmp/claude-<uid>)"
pol 2 '{"env": {"TMPDIR": "@S/tmp"}}'
attempt 2 "TMPDIR set (expect the same EPERM)"
pol 3 "{\"env\": {$tmpdir}}"
attempt 3 "CLAUDE_CODE_TMPDIR set, no credential (claude starts; the run hit a stale copied file here)"
pol 4 "{\"keychain\": true, \"env\": {$tmpdir}}"
attempt 4 "keychain allowed (expect Not logged in)"
pol 6 "{\"keychain\": true, \"env\": {$tmpdir, \"HOME\": \"@HOME\", \"USER\": \"@USER\"}}"
attempt 6 "keychain, real HOME, USER (expect Not logged in: hashed service name)"
pol 7 "{\"keychain\": true, \"env\": {$tmpdir, \"HOME\": \"@HOME\", \"USER\": \"@USER\", \"CLAUDE_SECURESTORAGE_CONFIG_DIR\": \"@S/login\"}}"
attempt 7 "keychain route, CLAUDE_SECURESTORAGE_CONFIG_DIR at the scratch login (the run used \"\" for the owner's item)"

# lease route: the scratch grant written into the config dir, never displayed
(umask 077; security find-generic-password -s "$(svc "$s/login")" -w > "$s/config/.credentials.json")
pol 8 "{\"env\": {$tmpdir, \"TMPDIR\": \"@S/tmp\"}}"
attempt 8 "keychain=false, HOME=state/home, leased .credentials.json"

echo hello-from-host > "$s/www/index.html"
python3 -m http.server 18765 --bind 127.0.0.1 --directory "$s/www" 2> "$out/http.log" &
srv=$!
sleep 1
cat > "$s/config/settings.json" <<'EOF'
{
  "hooks": {
    "Stop": [
      { "hooks": [ { "type": "command", "command": "/usr/bin/curl -s --max-time 3 http://127.0.0.1:18765/hook-stop" } ] }
    ]
  }
}
EOF
cp "$s/policy-8.json" "$s/policy-9.json"
attempt 9 "leased file policy + Stop hook curling the host listener"
{ echo "== host http.server log lines mentioning hook-stop:"; grep hook-stop "$out/http.log" || true; } \
  | tee -a "$out/test-c-claude-9.log"
rm "$s/config/settings.json"

pol 10 "{\"readonly\": [\"/nix/store\", \"@S/work\"], \"readwrite\": [\"@S/config\", \"@S/tmp\"], \"ingress\": null, \"hostLoopback\": null, \"keychain\": null, \"env\": {$tmpdir}}"
attempt 10 "minimal policy (no TMPDIR, no ingress, work ro, HOME ungranted)"
nix path-info -r "$(dirname "$bindir")" > "$s/closure"
pol 11 "{\"readonly\": \"@CLOSURE\", \"readwrite\": [\"@S/config\", \"@S/tmp\"], \"ingress\": null, \"hostLoopback\": null, \"keychain\": null, \"env\": {$tmpdir}}"
attempt 11 "minimal policy with /nix/store narrowed to the claude closure ($(wc -l < "$s/closure") paths)"

# transcripts written inside are readable outside
find "$s/config" -name '*.jsonl' -exec ls -l {} + | tee "$out/test-c-transcript-outside.log"
