import sys, json, urllib.request, time
tok = json.loads(sys.stdin.read())['claudeAiOauth']['accessToken']
req = urllib.request.Request('https://api.anthropic.com/api/oauth/usage', headers={
    'Authorization': 'Bearer ' + tok, 'anthropic-beta': 'oauth-2025-04-20', 'Content-Type': 'application/json',
    'User-Agent': 'rc2s2-spike/0'})
t = time.time()
try:
    with urllib.request.urlopen(req, timeout=10) as r:
        body = r.read(); status = r.status; hdr = dict(r.headers)
except urllib.error.HTTPError as e:
    body = e.read(); status = e.code; hdr = dict(e.headers)
print('status', status, 'ms', round((time.time()-t)*1000), 'ratelimit headers', [k for k in hdr if 'ratelimit' in k.lower() or 'retry' in k.lower()])
d = json.loads(body)
def shape(o):
    if isinstance(o, dict): return {k: shape(v) for k, v in o.items()}
    if isinstance(o, list): return [shape(o[0])] if o else []
    return o
print(json.dumps(shape(d), indent=1)[:3000])
