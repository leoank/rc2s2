#!/usr/bin/env bash
# RUN-20261016-rq14ag: two user turns over stream-json stdin into one `claude -p` process.
# Written on 2026-11-02 from the run's notes, not run at the time.
# multiturn.py sends "remember pelican", waits 10 s idle, asks for the word back, then
# closes stdin; stream.jsonl, stderr.txt and timeline.txt land in out/ beside this script.
# Needs the host's `claude` on PATH and a logged-in account (two small turns).
# From the repo root:
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14ag/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spikes=spikes

python3 "$spikes/05-headless/multiturn.py" "$here/out"
cat "$here/out/timeline.txt"
