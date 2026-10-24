#!/usr/bin/env python3
"""Poll ioreg PerformanceStatistics every INTERVAL s for DURATION s (EXP-20261016-rq1409)."""
import re, subprocess, sys, time
dur = float(sys.argv[1]) if len(sys.argv) > 1 else 15
step = float(sys.argv[2]) if len(sys.argv) > 2 else 0.5
keys = ["Device Utilization %", "Renderer Utilization %", "Tiler Utilization %",
        "In use system memory", "Alloc system memory"]
print("t_s\t" + "\t".join(keys))
t0 = time.perf_counter()
while (t := time.perf_counter() - t0) < dur:
    out = subprocess.run(["ioreg", "-r", "-d", "1", "-c", "IOAccelerator"],
                         capture_output=True, text=True).stdout
    m = re.search(r'"PerformanceStatistics" = \{(.*?)\}', out)
    stats = dict(re.findall(r'"([^"]+)"=(\d+)', m.group(1)))
    print(f"{t:5.1f}\t" + "\t".join(stats.get(k, "?") for k in keys), flush=True)
    time.sleep(step)
