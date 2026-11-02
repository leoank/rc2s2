#!/usr/bin/env bash
# RUN-20261016-qqc0qw: aarch64-darwin build of spikes/06-libtailscale (libtailscale as a Go
# c-archive, linked into a Rust binary) and a sandboxed run of the binary.
# Written on 2026-11-02 from the run's notes, not run at the time.
# Uses the spike's own flake and flake.lock as committed (the `nix flake lock` and the
# lib.fakeHash round that gave the vendorHash are not repeated). Build logs and run output
# land in out/ beside this script.
# Needs only nix with flakes on an aarch64-darwin host, and the network for the source
# fetch and the Go modules unless they are already in the store. No dev shell; from the
# repo root:
#   notes/runs/RUN-20261016-qqc0qw/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/out"
mkdir -p "$out"
cd spikes/06-libtailscale

lib="$(nix build .#libtailscale --no-link --print-build-logs --print-out-paths 2> "$out/darwin-libtailscale-build.log")"
bin="$(nix build .#default --no-link --print-build-logs --print-out-paths 2> "$out/darwin-ts-hello-build.log")"
run="$(nix build .#checks.aarch64-darwin.run --no-link --print-out-paths)"

{
  ls -l "$lib/lib/libtailscale.a" "$bin/bin/ts-hello"
  nm "$lib/lib/libtailscale.a" 2>/dev/null | grep -c ' T _tailscale_' || true
  otool -L "$bin/bin/ts-hello"
  echo "sandboxed:"; cat "$run"
  echo "direct:"; "$bin/bin/ts-hello"
} | tee "$out/darwin-run-output.txt"
