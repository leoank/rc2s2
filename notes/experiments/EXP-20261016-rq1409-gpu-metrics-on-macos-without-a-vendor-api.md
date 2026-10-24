---
id: EXP-20261016-rq1409
previous_id: manthan/EXP-0009
type: experiment
schema_version: 1
title: GPU metrics on macOS without a vendor API
project: PRJ-0001
status: concluded           # planned | active | paused | concluded | abandoned
outcome: positive           # null | positive | negative | inconclusive | infra
influenced_by: []
created: 2026-10-16
author: ank
protocols: []
materials: []
tags: [observation, phase-0, health, gpu, macos]
review: approved
---

## Question
Which GPU utilisation and memory metrics can be read on macOS without sudo and without a
vendor API, cheaply enough for a 30 second health sample? Fifth open question.

## Hypothesis and predictions
`ioreg -c IOAccelerator` exposes `PerformanceStatistics` with device utilisation and
in-use memory, readable without privileges; `powermetrics` gives better numbers but
needs sudo; `system_profiler` is static. A Rust daemon can read the same keys through
IOKit, so the health collector does not need to shell out.

## Plan
1. Dump `ioreg -r -d 1 -c IOAccelerator` and list the statistics keys and their
   meaning.
2. Sample under idle and under load and check the numbers move.
3. Time the call; try the same read through IOKit from a short Rust program.
4. Note what Linux gives for comparison: NVML or `nvidia-smi`, `amdgpu` sysfs.

## Success criteria
- Utilisation and memory in use readable without sudo, with the exact keys listed.
- A sampling cost well under 100 ms.

## Findings
Apple M3 Max, macOS 26.5.2; details, dumps and scripts in [[RUN-20261016-rq14r0]] and
`experiments/EXP-20261016-rq1409/`.

**Source.** One IORegistry entry of class `AGXAcceleratorG15X` matches both
`IOAccelerator` and `AGXAccelerator` (`IOMatchCategory = IOAccelerator`). Its
`PerformanceStatistics` dictionary is readable by any user. Keys on this driver:

| Key | Meaning |
|-----|---------|
| `Device Utilization %` | Share of the sample window the GPU had work in flight; the headline number |
| `Renderer Utilization %` | Fragment/render stage busy share; 0 under pure compute |
| `Tiler Utilization %` | Vertex/tiling stage busy share; 0 under pure compute |
| `In use system memory` | Bytes of unified memory currently held by GPU resources, system-wide |
| `In use system memory (driver)` | Driver-internal part of the above (0 here) |
| `Alloc system memory` | Bytes mapped for GPU use, >= in use (~4.1-4.4 GB here) |
| `Allocated PB Size` | Tiling parameter-buffer bytes allocated (148 MB) |
| `TiledSceneBytes` | Geometry bytes tiled in the last render; 0 at idle |
| `SplitSceneCount` | Renders that overflowed the parameter buffer and were split |
| `recoveryCount` | GPU resets since boot; a health signal in its own right |
| `lastRecoveryTime` | Timestamp of the last reset (0 = none) |

There is no "total GPU memory": memory is unified, so the ceiling is `hw.memsize`.

**Numbers.** Under an 8 s Metal compute load `Device Utilization %` goes 0 -> 98-100 ->
0 in step with the load; Renderer/Tiler stay 0 except for window-server blips (18-37).
`In use system memory` ranged 175-576 MB and is not per-process. Idle reads show 0-1 %.

**Cost.** `ioreg -r -d 1 -c IOAccelerator`: ~10 ms wall, ~18 ms through fork/exec.
In-process through IOKit from Rust: first sample 140-280 us (includes service matching),
then 37-53 us per full sample. Both meet the <100 ms criterion; the IOKit path is
200-500x cheaper and needs no parsing of `ioreg` text.

**Other sources.** `powermetrics` refuses without root ("must be invoked as the
superuser"). `system_profiler SPDisplaysDataType` is static: chipset, 30 cores, vendor
id, Metal 4, displays. The accelerator entry also publishes an `IOReportLegend` with "GPU
Energy" and "GPU Stats" channels; that is the private IOReport route asitop/macmon use
and is not needed for utilisation or memory.

**Recommended approach for the daemon.** `io-kit-sys` 0.4 + `core-foundation` 0.10
(both compile unchanged with the nixpkgs `apple-sdk`, no bindgen), as in
`code/rc2s2-.../spikes/09-gpu-macos/src/main.rs`:
`IOServiceMatching("IOAccelerator")` -> `IOServiceGetMatchingServices` -> for each
service `IORegistryEntryCreateCFProperty(svc, "PerformanceStatistics")` -> read the
`CFNumber`s for `Device Utilization %` and `In use system memory` (plus
`recoveryCount` for health). Iterate all services so Intel/AMD Macs with more than one
accelerator are covered; their drivers' key sets were not tested here. `objc2-io-kit`
exposes the same calls and would also work; not tried. On Linux: `nvml-wrapper` for
NVIDIA (`utilization_rates`, `memory_info`), sysfs `gpu_busy_percent` /
`mem_info_vram_used` / `mem_info_vram_total` for amdgpu, both unprivileged.

## Runs
- [[RUN-20261016-rq14r0]] idle vs Metal load, ioreg and Rust IOKit timings

## Conclusion
- use iokit
