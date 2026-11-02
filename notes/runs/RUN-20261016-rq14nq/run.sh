#!/usr/bin/env bash
# RUN-20261016-rq14nq: build the mxc-sdk spike under Nix on macOS, first with cargo in the
# spike's dev shell, then with `nix build`, and run the nix-built binary on /bin/echo under an
# empty seatbelt policy. Logs go to build-devshell.log, build-nix.log and
# test-a-echo-nix-built.log beside this script.
# Written on 2026-11-02 from the run's notes; it was not run at the time.
# From the repo root, in the spike's dev shell (macOS, nix able to fetch crates):
#   nix develop ./spikes/07-mxc -c \
#     notes/runs/RUN-20261016-rq14nq/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
spike=spikes/07-mxc
exp=notes/experiments/EXP-20261016-rq1407

# 1. dev shell build
( time cargo build --manifest-path "$spike/Cargo.toml" ) 2>&1 | tee "$here/build-devshell.log"

# 2. nix build (rustPlatform.buildRustPackage + cargoLock); the flake reads the git-tracked tree
( time nix build "./$spike" --no-link --print-out-paths -L ) 2>&1 | tee "$here/build-nix.log"
out="$(nix build "./$spike" --no-link --print-out-paths)"

# 3. the nix-built binary sandboxes /bin/echo with an empty policy
"$out/bin/mxc-spike" "$exp/policy-echo.json" -- /bin/echo hi-from-nix-built 2>&1 \
  | tee "$here/test-a-echo-nix-built.log"
