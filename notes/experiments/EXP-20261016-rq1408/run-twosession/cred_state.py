import json, hashlib, sys, time, os
# print non-secret state of a credentials json file: key set, hashes of tokens (8 hex), expiry times
p = sys.argv[1]
try:
    raw = open(p).read()
except FileNotFoundError:
    print(p, 'MISSING'); sys.exit()
d = json.loads(raw); o = d.get('claudeAiOauth', {})
h = lambda s: hashlib.sha256(s.encode()).hexdigest()[:8] if s else repr(s)
now = time.time()*1000
print(os.path.basename(os.path.dirname(p)) or p, '| keys', sorted(o.keys()), '| access', h(o.get('accessToken')), '| refresh', h(o.get('refreshToken')),
      '| expiresAt', o.get('expiresAt'), f"({(o.get('expiresAt') or 0 - now)/60000 if o.get('expiresAt') else 0:+.0f}m)",
      '| refreshTokenExpiresAt', o.get('refreshTokenExpiresAt'), '| mtime', time.strftime('%H:%M:%S', time.localtime(os.path.getmtime(p))))
