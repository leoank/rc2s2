#!/usr/bin/env bash
# RUN-20261016-rq14r0: GPU utilisation and memory on macOS from the IOAccelerator registry
# entry, idle and under a Metal compute load, by ioreg and by an in-process IOKit read.
# Written on 2026-11-02 from the run's notes, not run at the time.
# Builds gpu_load.m (from the EXP-20261016-rq1409 sidecar) into a temporary directory and
# spikes/09-gpu-macos with `cargo build --release`, both in the spike's dev shell; samples
# ioreg every 0.5 s for 16 s with the load from t=3 s for 8 s; times ioreg and the Rust read;
# tries powermetrics without sudo and system_profiler. Outputs land in out/ beside this script.
# Needs an Apple Silicon Mac, no sudo, no Xcode. From the repo root, in the project shell
# (for python3; the script enters the spike's own shell for clang and cargo):
#   nix develop . -c \
#     notes/runs/RUN-20261016-rq14r0/run.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/out"
mkdir -p "$out"
spike=spikes/09-gpu-macos
exp=notes/experiments/EXP-20261016-rq1409
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nix develop "./$spike" -c clang -fobjc-arc -framework Metal -framework Foundation \
  "$exp/gpu_load.m" -o "$tmp/gpu_load"
nix develop "./$spike" -c cargo build --release --manifest-path "$spike/Cargo.toml"
gpu="$spike/target/release/gpu-macos"

# 1. Registry dump at idle.
ioreg -r -d 1 -c IOAccelerator > "$out/ioreg-IOAccelerator-idle.txt"
ioreg -r -d 1 -c AGXAccelerator > "$tmp/agx.txt"
head -40 "$tmp/agx.txt" > "$out/ioreg-AGXAccelerator-head.txt"

# 2. Idle vs load.
python3 "$exp/sample_ioreg.py" 16 0.5 > "$out/ioreg-samples-idle-load.tsv" &
sampler=$!
sleep 3
"$tmp/gpu_load" 8
wait "$sampler"

# 3. Timing: ioreg wall time, via fork/exec, and the in-process read idle and under load.
for _ in 1 2 3; do /usr/bin/time -p ioreg -r -d 1 -c IOAccelerator > /dev/null; done 2> "$out/ioreg-time.txt"
python3 -c 'import subprocess, time
t = time.perf_counter()
for _ in range(10): subprocess.run(["ioreg", "-r", "-d", "1", "-c", "IOAccelerator"], capture_output=True)
print(f"mean {(time.perf_counter() - t) / 10 * 1000:.1f} ms incl. fork/exec")' | tee -a "$out/ioreg-time.txt"
"$gpu" > "$out/rust-idle.txt"
"$tmp/gpu_load" 8 &
load=$!
sleep 3
"$gpu" > "$out/rust-under-load.txt"
wait "$load"

# 4. Other sources.
{ powermetrics --samplers gpu_power -n 1 2>&1 || echo "exit status: $?"; } > "$out/powermetrics-no-sudo.txt"
system_profiler SPDisplaysDataType > "$out/system_profiler-SPDisplaysDataType.txt"
cat "$out/rust-idle.txt" "$out/rust-under-load.txt"
