# non-secret view and manipulation of the credential store of one config dir (macOS):
#   store.py state <dir>      print key set, token hashes, expiries (file if present, else keychain item)
#   store.py backdate <dir>   set expiresAt one hour into the past, in place (file or keychain item)
#   store.py delete <dir>     delete the keychain item
# never prints a token value
import hashlib, json, os, subprocess, sys, time
cmd, cfg = sys.argv[1], sys.argv[2]
svc = "Claude Code-credentials-" + hashlib.sha256(cfg.encode()).hexdigest()[:8]
path = os.path.join(cfg, ".credentials.json")
h = lambda s: hashlib.sha256(s.encode()).hexdigest()[:8] if s else repr(s)

def read():
    if os.path.exists(path):
        return "file", open(path).read()
    r = subprocess.run(["security", "find-generic-password", "-s", svc, "-w"], capture_output=True, text=True)
    return ("keychain", r.stdout.strip()) if r.returncode == 0 else ("none", "")

def show(where, raw):
    if not raw:
        print(os.path.basename(cfg), "|", where, svc, "| EMPTY"); return
    o = json.loads(raw).get("claudeAiOauth", {})
    now = time.time() * 1000
    exp = o.get("expiresAt") or 0
    print(os.path.basename(cfg), "|", where, "| access", h(o.get("accessToken")), "| refresh", h(o.get("refreshToken")),
          "| expiresAt", exp, f"({(exp - now) / 60000:+.0f}m)", "| refreshTokenExpiresAt", o.get("refreshTokenExpiresAt"))

where, raw = read()
if cmd == "state":
    show(where, raw)
elif cmd == "backdate":
    d = json.loads(raw)
    d["claudeAiOauth"]["expiresAt"] = int(time.time() * 1000) - 3600000
    new = json.dumps(d)
    if where == "file":
        open(path, "w").write(new)
    else:
        subprocess.run(["security", "add-generic-password", "-a", os.environ["USER"], "-s", svc, "-w", new, "-U"], check=True)
    show(*read())
elif cmd == "delete":
    print(subprocess.run(["security", "delete-generic-password", "-s", svc], capture_output=True, text=True).returncode)
