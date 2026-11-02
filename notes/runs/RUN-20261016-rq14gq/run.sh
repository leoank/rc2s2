#!/usr/bin/env bash
# RUN-20261016-rq14gq: headless resume (`claude -p --resume <id>`) of a session started at a terminal.
# Written on 2026-11-02 from the run's notes, not run at the time.
# The original resumed the interactive session of RUN-20261016-rq141j. Here a fresh
# interactive session is started under spikes/01-hooks/pty_driver.py with a known
# --session-id, asked to reply `pelican`, and exited; then it is resumed in print mode.
# The session runs in a scratch work/ beside this script (inside the repo, so the
# workspace is already trusted), removed at the end. Outputs land in out/.
# Needs the host's `claude` on PATH and a logged-in account (two small turns).
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14gq/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
driver="$PWD/spikes/01-hooks/pty_driver.py"
out="$here/out"
work="$here/work"
mkdir -p "$out" "$work"
trap 'rm -rf "$work"' EXIT
sid="$(python3 -c 'import uuid; print(uuid.uuid4())')"

cd "$work"
python3 "$driver" "$out/interactive.pty" \
  '[{"wait": "shortcuts", "timeout": 30}, {"send": "Reply with just the word pelican."}, {"sleep": 1},
    {"send": "\r"}, {"sleep": 20}, {"send": "/exit\r"}, {"sleep": 3}]' \
  -- claude --session-id "$sid"

# The original prompt is cut off in the notes ("What single word..."); this is a stand-in.
env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT -u AI_AGENT claude -p --resume "$sid" \
  "What single word did you reply with last? Reply with just that word." \
  --output-format json > "$out/resume.json" 2> "$out/resume.stderr"
python3 -c 'import json, sys; d = json.load(open(sys.argv[1])); print(d["result"], d["session_id"], d["num_turns"])' \
  "$out/resume.json"
echo "expected: pelican $sid 1"
