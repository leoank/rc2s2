#!/usr/bin/env python3
"""EXP-0002: tabulate the line shapes of Claude Code transcript JSONL files.

Walks ~/.claude/projects/**/*.jsonl and prints a markdown report to stdout.
Privacy: values are never printed. Every skeleton replaces strings with "string",
numbers with 123, booleans with true. Only key names, type names, counts and the
values of a short whitelist of enum-like keys (type, role, stop_reason, model, ...)
appear in the output. Project directory names (which encode cwd paths) are replaced
by an index.
"""
import glob
import json
import os
import re
import sys
from collections import Counter, defaultdict

ROOT = os.path.expanduser("~/.claude/projects")

# keys whose (short, non-path) values are enum-like and safe to list
ENUM_KEYS = {
    "type", "subtype", "role", "stop_reason", "model", "entrypoint", "userType",
    "permissionMode", "promptSource", "turnOrigin", "level", "operation", "hookEvent",
    "service_tier", "speed", "inference_geo", "version", "renderedRole", "decision",
    "isSidechain", "is_error", "hasUnknownModelCost", "preventedContinuation",
    "isMeta", "isCompactSummary", "isVisibleInTranscriptOnly", "status",
}
USAGE_RE = re.compile(r"token|cost|usage|usd|duration", re.I)
RATE_RE = re.compile(r"rate|limit|quota|overage|utilization|resets?_?at|window|tier", re.I)
CHAIN_KEYS = {"uuid", "parentUuid", "isSidechain", "agentId", "sessionId", "promptId",
              "sourceToolAssistantUUID", "leafUuid", "toolUseID", "requestId",
              "timestamp", "cwd", "version", "gitBranch"}
ID_RE = re.compile(r"^(toolu_|msg_|req_|agent-|[0-9a-f]{8}-[0-9a-f]{4}-)")
PATH_RE = re.compile(r"[/\\]|\.[a-z0-9]{1,5}$")


def key_name(k):
    """Collapse dict keys that are ids or file paths; both are values in disguise."""
    if ID_RE.match(k):
        return "<id>"
    if PATH_RE.search(k):
        return "<path>"
    return k
MAX_DEPTH = 7


def redact(v):
    if isinstance(v, bool):
        return True
    if isinstance(v, (int, float)):
        return 123
    if v is None:
        return None
    if isinstance(v, str):
        return "string"
    raise TypeError


def merge(a, b, depth=0):
    """Merge skeleton b into skeleton a (union of keys, one element per list)."""
    if depth > MAX_DEPTH:
        return "..."
    if isinstance(b, dict):
        if not isinstance(a, dict):
            a = {}
        for k, v in b.items():
            kk = key_name(k)
            a[kk] = merge(a.get(kk), v, depth + 1)
        return a
    if isinstance(b, list):
        inner = a[0] if isinstance(a, list) and a else None
        for x in b:
            inner = merge(inner, x, depth + 1)
        return [inner] if b or not isinstance(a, list) else a
    r = redact(b)
    if a is None or a == r:
        return r
    if isinstance(a, str) and a.startswith("<"):
        s = set(a[1:-1].split("|"))
    else:
        s = {json.dumps(a)}
    s.add(json.dumps(r))
    return "<" + "|".join(sorted(s)) + ">"


def walk_keys(o, path, out):
    """Collect dotted key paths matching the usage / rate-limit regexes."""
    if isinstance(o, dict):
        for k, v in o.items():
            kk = key_name(k)
            p = f"{path}.{kk}" if path else kk
            out.add(p)
            walk_keys(v, p, out)
    elif isinstance(o, list):
        for x in o:
            walk_keys(x, path + "[]", out)


def classify(path):
    rel = os.path.relpath(path, ROOT)
    parts = rel.split(os.sep)
    if "subagents" in parts:
        return "subagent"
    if len(parts) == 2:
        return "main"
    return "other"


def main():
    files = sorted(glob.glob(os.path.join(ROOT, "**", "*.jsonl"), recursive=True))
    projects = sorted({os.path.relpath(f, ROOT).split(os.sep)[0] for f in files})
    pidx = {p: i for i, p in enumerate(projects)}

    shape = {}                         # (class, type-key) -> merged skeleton
    count = Counter()                  # (class, type-key) -> lines
    keys = defaultdict(Counter)        # (class, type-key) -> top-level key -> lines
    blocks = {}                        # block type -> merged skeleton
    bcount = Counter()                 # (role, block type) -> count
    enums = defaultdict(Counter)       # key -> value -> count
    usage_paths, rate_paths = set(), set()
    per_file = []
    bad = Counter()
    meta_shape = None

    for f in files:
        cls = classify(f)
        stem = os.path.splitext(os.path.basename(f))[0]
        n = 0
        types = Counter()
        uuids, parents = set(), []
        sidechain = 0
        entrypoints, versions = set(), set()
        has_cwd = has_sid = 0
        sid_match = 0
        agent_ids = set()
        assistant_lines = 0
        msg_ids = set()
        first_ts = last_ts = None
        cost_state = 0
        with open(f) as fh:
            for raw in fh:
                raw = raw.strip()
                if not raw:
                    continue
                n += 1
                try:
                    o = json.loads(raw)
                except json.JSONDecodeError:
                    bad[cls] += 1
                    continue
                t = o.get("type", "<none>")
                tk = t
                if t == "attachment" and isinstance(o.get("attachment"), dict):
                    tk = f"attachment/{o['attachment'].get('type')}"
                elif t == "system":
                    tk = f"system/{o.get('subtype')}"
                types[tk] += 1
                count[(cls, tk)] += 1
                shape[(cls, tk)] = merge(shape.get((cls, tk)), o)
                for k in o:
                    keys[(cls, tk)][k] += 1
                # enum-ish values
                for k in ENUM_KEYS:
                    if k in o and isinstance(o[k], (str, bool)) and len(str(o[k])) < 40:
                        enums[k][str(o[k])] += 1
                m = o.get("message")
                if isinstance(m, dict):
                    for k in ("role", "stop_reason", "model"):
                        if k in m and isinstance(m[k], str):
                            enums["message." + k][m[k]] += 1
                    c = m.get("content")
                    if isinstance(c, str):
                        bcount[(m.get("role"), "<string>")] += 1
                    elif isinstance(c, list):
                        for b in c:
                            if isinstance(b, dict):
                                bt = b.get("type", "<none>")
                                bcount[(m.get("role"), bt)] += 1
                                blocks[bt] = merge(blocks.get(bt), b)
                    if t == "assistant":
                        assistant_lines += 1
                        if "id" in m:
                            msg_ids.add(m["id"])
                    u = m.get("usage")
                    if isinstance(u, dict):
                        for k in ("service_tier", "speed", "inference_geo"):
                            if isinstance(u.get(k), str):
                                enums["usage." + k][u[k]] += 1
                if t == "cost-state":
                    cost_state += 1
                # usage / rate-limit key paths
                ks = set()
                walk_keys(o, "", ks)
                for p in ks:
                    leaf = p.rsplit(".", 1)[-1]
                    if USAGE_RE.search(leaf):
                        usage_paths.add((tk, p))
                    if RATE_RE.search(leaf):
                        rate_paths.add((tk, p))
                # chaining
                if "uuid" in o:
                    uuids.add(o["uuid"])
                if "parentUuid" in o:
                    parents.append(o["parentUuid"])
                if o.get("isSidechain"):
                    sidechain += 1
                if "agentId" in o:
                    agent_ids.add(o["agentId"])
                if "entrypoint" in o:
                    entrypoints.add(o["entrypoint"])
                if "version" in o:
                    versions.add(o["version"])
                if "cwd" in o:
                    has_cwd += 1
                if "sessionId" in o:
                    has_sid += 1
                    if o["sessionId"] == stem:
                        sid_match += 1
                ts = o.get("timestamp")
                if isinstance(ts, str):
                    first_ts = first_ts or ts
                    last_ts = ts
        dangling = sum(1 for p in parents if p is not None and p not in uuids)
        per_file.append(dict(
            project=pidx[os.path.relpath(f, ROOT).split(os.sep)[0]], cls=cls,
            lines=n, types=len(types), entry="|".join(sorted(entrypoints)) or "-",
            version="|".join(sorted(versions)) or "-", cwd=has_cwd, sid=has_sid,
            sid_match=sid_match, uuids=len(uuids), parents=len(parents),
            dangling=dangling, sidechain=sidechain, agents=len(agent_ids),
            assistant=assistant_lines, msgs=len(msg_ids), cost_state=cost_state,
            span=f"{first_ts[:16] if first_ts else '-'} .. {last_ts[:16] if last_ts else '-'}",
        ))
        # subagent meta file beside the transcript
        if cls == "subagent":
            mp = f[:-len(".jsonl")] + ".meta.json"
            if os.path.exists(mp):
                try:
                    meta_shape = merge(meta_shape, json.load(open(mp)))
                except Exception:
                    pass

    # non-transcript files under projects/
    others = Counter()
    for dp, _, fns in os.walk(ROOT):
        for fn in fns:
            if not fn.endswith(".jsonl"):
                rel = os.path.relpath(dp, ROOT).split(os.sep)
                where = rel[1] if len(rel) > 1 else "."
                if len(rel) > 2:
                    where = "<session>/" + rel[2]
                others[(where, os.path.splitext(fn)[1] or fn)] += 1

    out = sys.stdout
    w = out.write
    w("# Transcript JSONL shapes\n\n")
    w(f"Generated by `spikes/02-transcript/shapes.py` over `~/.claude/projects` "
      f"({len(projects)} project dirs, {len(files)} jsonl files, "
      f"{sum(count.values())} lines, {sum(bad.values())} unparsable).\n\n")
    w("Values are redacted: strings are `\"string\"`, numbers `123`, booleans `true`. "
      "Dict keys that are ids (`toolu_...`, uuids, `agent-...`) are shown as `<id>`, keys that are file paths as `<path>`. "
      "A list shows the merged skeleton of its elements as one element. "
      "`<a|b>` marks a key seen with more than one scalar type.\n\n")

    w("## Files\n\n")
    w("| # | project | class | lines | types | entrypoint | version | cwd | sessionId | sid==file | uuids | parents | dangling | sidechain | agentIds | assistant lines | unique msg ids | cost-state | span (UTC) |\n")
    w("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n")
    for i, r in enumerate(per_file):
        w(f"| {i} | {r['project']} | {r['cls']} | {r['lines']} | {r['types']} | {r['entry']} | {r['version']} | "
          f"{r['cwd']} | {r['sid']} | {r['sid_match']} | {r['uuids']} | {r['parents']} | {r['dangling']} | "
          f"{r['sidechain']} | {r['agents']} | {r['assistant']} | {r['msgs']} | {r['cost_state']} | {r['span']} |\n")
    w("\n`dangling` = parentUuid values that do not point at a uuid in the same file. "
      "`sid==file` = lines whose sessionId equals the file stem.\n\n")

    w("## Non-jsonl files under a project dir\n\n")
    w("| location | ext | count |\n|---|---|---|\n")
    for (where, ext), c in sorted(others.items()):
        w(f"| {where} | {ext} | {c} |\n")

    w("\n## Line types\n\n")
    w("| class | type | lines | top-level keys (count) |\n|---|---|---|---|\n")
    for (cls, tk), c in sorted(count.items()):
        ks = ", ".join(f"{k}({v})" if v != c else k for k, v in sorted(keys[(cls, tk)].items()))
        w(f"| {cls} | `{tk}` | {c} | {ks} |\n")

    w("\n## Content block types (message.content[])\n\n")
    w("| role | block type | count |\n|---|---|---|\n")
    for (role, bt), c in sorted(bcount.items(), key=lambda x: (str(x[0][0]), str(x[0][1]))):
        w(f"| {role} | `{bt}` | {c} |\n")

    w("\n## Enum-like values\n\n")
    for k, vc in sorted(enums.items()):
        w(f"- `{k}`: " + ", ".join(f"`{v}` ({c})" for v, c in sorted(vc.items())) + "\n")

    w("\n## Keys carrying usage / cost / duration\n\n")
    for tk, p in sorted(usage_paths):
        w(f"- `{tk}`: `{p}`\n")
    w("\n## Keys that look like rate-limit / quota state\n\n")
    if rate_paths:
        for tk, p in sorted(rate_paths):
            w(f"- `{tk}`: `{p}`\n")
    else:
        w("(none)\n")

    w("\n## Chaining and identity keys per line type\n\n")
    w("| class | type | " + " | ".join(sorted(CHAIN_KEYS)) + " |\n")
    w("|---|---|" + "---|" * len(CHAIN_KEYS) + "\n")
    for (cls, tk), c in sorted(count.items()):
        w(f"| {cls} | `{tk}` | " + " | ".join(
            ("y" if keys[(cls, tk)].get(k, 0) == c else (f"{keys[(cls, tk)][k]}/{c}" if keys[(cls, tk)].get(k) else "-"))
            for k in sorted(CHAIN_KEYS)) + " |\n")

    w("\n## Skeletons per line type (merged, redacted)\n\n")
    for (cls, tk), sk in sorted(shape.items()):
        w(f"### {cls} / `{tk}` ({count[(cls, tk)]} lines)\n\n```json\n{json.dumps(sk, indent=1)}\n```\n\n")

    w("## Skeletons per content block type\n\n")
    for bt, sk in sorted(blocks.items()):
        w(f"### `{bt}`\n\n```json\n{json.dumps(sk, indent=1)}\n```\n\n")

    if meta_shape is not None:
        w("## Subagent `agent-*.meta.json` skeleton\n\n```json\n" + json.dumps(meta_shape, indent=1) + "\n```\n")


if __name__ == "__main__":
    main()
