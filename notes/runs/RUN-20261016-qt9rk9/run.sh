#!/usr/bin/env bash
# RUN-20261016-qt9rk9: x86_64-linux build of spikes/06-libtailscale and a sandboxed run of
# the binary, submitted from the darwin host to the configured remote builder.
# Written on 2026-11-02 from the run's notes, not run at the time.
# One `nix build` of the linux check builds libtailscale, then ts-hello, then runs it in the
# builder's sandbox. The full nix log and the run output land in out/ beside this script.
# Needs nix with flakes and a remote x86_64-linux builder in /etc/nix/machines (the original
# used ssh-ng://ank@spirit), plus the network for the source fetch and Go modules. No dev
# shell; from the repo root:
#   notes/runs/RUN-20261016-qt9rk9/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/out"
mkdir -p "$out"
cd spikes/06-libtailscale

run="$(nix build .#checks.x86_64-linux.run --no-link --print-build-logs --print-out-paths 2> "$out/linux-build.log")"
cat "$run" | tee "$out/linux-run-output.txt"
# by hand: the sizes and `patchelf --print-needed` in the notes were read off the linux
# libtailscale and ts-hello store paths named in linux-build.log.
