#!/usr/bin/env bash
# RUN-20261019-73b0sc: the hub's record test, repeated.
# Runs crates/hub/tests/record.rs (a real hub and a real node link on loopback: hub restart
# with buffered events, node restart, unknown token) N times and writes a summary to
# tests.txt beside this script. Touches nothing outside a temp directory per test.
# From the repo root:
#   nix develop ./code/rc2s2-a-remote-command-and-control-session-server -c \
#     notes/projects/PRJ-0002-rc2s2-a-remote-command-and-control-session-server/runs/RUN-20261019-73b0sc/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
manifest=code/rc2s2-a-remote-command-and-control-session-server/Cargo.toml
n="${RC2S2_REPEAT:-20}"

cargo test --quiet --manifest-path "$manifest" -p rc2s2-hub --test record --no-run 2>/dev/null
for i in $(seq "$n"); do
  if ! out="$(cargo test --quiet --manifest-path "$manifest" -p rc2s2-hub --test record 2>&1)"; then
    printf 'failure on repeat %s:\n%s\n' "$i" "$out" | grep -E 'panicked|FAILED|failure' || true
  fi
done | tee "$here/failures.tmp"
{
  echo "rustc: $(rustc --version)"
  echo "repeats: $n"
  echo "failed repeats: $(grep -c '^failure on repeat' "$here/failures.tmp" || true)"
  cat "$here/failures.tmp"
  cargo test --manifest-path "$manifest" -p rc2s2-hub --test record 2>&1 | grep -E '^test |test result'
} > "$here/tests.txt"
rm "$here/failures.tmp"
cat "$here/tests.txt"
