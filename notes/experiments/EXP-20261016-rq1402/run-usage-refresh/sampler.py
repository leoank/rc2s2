#!/usr/bin/env python3
"""Log ~/.claude.json mtime and cachedUsageUtilization.fetchedAtMs every 2 s, when they change.
Only key names and the two percentages are recorded; nothing else from the file.
Usage: sampler.py SECONDS OUTFILE
"""
import datetime, json, os, sys, time

secs, out = int(sys.argv[1]), sys.argv[2]
p = os.path.expanduser("~/.claude.json")
last = None
t_end = time.time() + secs
with open(out, "a") as f:
    while time.time() < t_end:
        try:
            st = os.stat(p)
            c = json.load(open(p)).get("cachedUsageUtilization")
            fa = c.get("fetchedAtMs") if c else None
            u = (c or {}).get("utilization") or {}
            fh = (u.get("five_hour") or {}).get("utilization")
            sd = (u.get("seven_day") or {}).get("utilization")
            acct = "acct" if c and "accountUuid" in c else "noacct"
        except Exception as e:  # file mid-rewrite
            st, fa, fh, sd, acct = None, f"ERR {type(e).__name__}", None, None, None
        cur = (round(st.st_mtime, 3) if st else None, fa, fh, sd, acct)
        if cur != last:
            now = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
            mt = datetime.datetime.fromtimestamp(st.st_mtime).astimezone().isoformat(timespec="seconds") if st else None
            fat = datetime.datetime.fromtimestamp(fa / 1000).astimezone().isoformat(timespec="seconds") if isinstance(fa, (int, float)) else fa
            f.write(f"{now} mtime={mt} fetchedAtMs={fa} ({fat}) five_hour={fh} seven_day={sd} {acct}\n")
            f.flush()
            last = cur
        time.sleep(2)
