#!/usr/bin/env bash
# RUN-20261019-ank8b1: insert cost of the hub's events table with and without a unique index
# on (node, session, transcript uuid), grown to RC2S2_ROWS rows (default 10 million) of
# synthetic data. Writes bench.txt beside this script; databases go in a temporary directory
# that is removed (about 4 GB at the default size, one variant at a time).
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-ank8b1/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
rows="${RC2S2_ROWS:-10000000}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
{
  python3 -I -c 'import sqlite3, platform; print(f"sqlite {sqlite3.sqlite_version}, python {platform.python_version()}, {platform.machine()}")'
  for variant in none text text-64mb blob; do
    python3 -I "$here/bench.py" "$variant" "$tmp/$variant.db" "$rows"
    rm -f "$tmp/$variant.db"*
  done
} | tee "$here/bench.txt"
