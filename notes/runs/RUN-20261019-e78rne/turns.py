"""RUN-20261019-e78rne (as RUN-20261019-e63842): the throwaway session's own transcript, turn by turn: what was
submitted, which tools ran, what the model said, with times. Reads one file given as the
argument; it belongs to the run's session only."""
import json, sys

for raw in open(sys.argv[1]):
    v = json.loads(raw)
    t, ts = v.get("type"), (v.get("timestamp") or "")[11:19]
    msg = v.get("message") or {}
    content = msg.get("content")
    if t == "user":
        if isinstance(content, str):
            print(f"{ts} user: {content[:160]!r}")
        else:
            for b in content or []:
                if b.get("type") == "tool_result":
                    r = b.get("content")
                    r = r if isinstance(r, str) else json.dumps(r)
                    print(f"{ts}   tool result: {r[:160]!r}")
                elif b.get("type") == "text":
                    print(f"{ts} user: {b['text'][:160]!r}")
    elif t == "assistant":
        for b in content or []:
            if b.get("type") == "text" and b["text"].strip():
                print(f"{ts} assistant: {b['text'].strip()[:200]!r}")
            elif b.get("type") == "tool_use":
                print(f"{ts}   tool use: {b['name']} {json.dumps(b.get('input'))[:160]}")
    elif t == "system":
        print(f"{ts} system: {v.get('subtype')} {str(v.get('content') or '')[:120]!r}")
