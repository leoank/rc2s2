#!/usr/bin/env bash
# RUN-20261016-rq14x8: background sessions (`claude --bg`, logs, agents, stop, rm) and the
# ~/.claude/sessions registry.
# Written on 2026-11-02 from the run's notes, not run at the time.
# Starts one background session in a scratch work/ beside this script (inside the repo, so
# the workspace is trusted; the original ran in spikes/01-hooks/work-pty), captures each
# command's output in out/, then stops and removes it. Lists the field names of the registry
# entries, never the contents of the <pid>.<hash>.key files. The original's stray turns from
# passing a wrong id to `logs`/`stop` are not repeated.
# Needs the host's `claude` on PATH and a logged-in account (one small turn).
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14x8/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/out"
work="$here/work"
mkdir -p "$out" "$work"
trap 'rm -rf "$work"' EXIT
cd "$work"

claude --help > "$out/bg-help.txt"
claude --bg -n exp05bg "Reply with just the word noted." | tee "$out/bg-start.txt"
id="$(awk '/^backgrounded/ {print $3}' "$out/bg-start.txt")"
sleep 15
claude logs "$id" > "$out/bg-logs.txt"
claude agents > "$out/bg-agents.txt" 2>&1 || true   # needs a TTY; the error is the finding
claude agents --json > "$out/bg-agents.json"
claude stop "$id" | tee "$out/bg-stop.txt"
claude rm "$id"

python3 - "$out/registry-fields.txt" <<'EOF'
import glob, json, os, sys
with open(sys.argv[1], "w") as f:
    for p in sorted(glob.glob(os.path.expanduser("~/.claude/sessions/*.json"))):
        e = json.load(open(p))
        try:
            os.kill(e["pid"], 0); alive = True
        except OSError:
            alive = False
        f.write(f"{os.path.basename(p)} alive={alive} status={e.get('status')} keys={sorted(e)}\n")
    keys = glob.glob(os.path.expanduser("~/.claude/sessions/*.key"))
    f.write(f"{len(keys)} key files beside the entries (contents not read)\n")
EOF
cat "$out/registry-fields.txt"
