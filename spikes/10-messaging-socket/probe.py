#!/usr/bin/env python3
"""Same-pid probe: send a user message (and optionally notify_when_idle) to a
target session, then serve the reply address from THIS process so the receiver's
pid-pinned receipt (peer_message_status) or idle notice (peer_idle_notice) can be
captured.

The receiver verifies the reply endpoint's peer pid (SO_PEERCRED / LOCAL_PEERPID)
against the pid that sent the original message, so the reply socket must be served
by the same process. This binds /tmp/cc-socks/<ownpid>.sock, registers it, sends
the frames with from=uds:<ownpid sock>, and accepts reply connections for a while.

Usage: probe.py --pid TARGET --text "..." [--notify] [--seconds 20]
"""
import argparse, json, os, socket, subprocess, sys, threading, time, uuid, glob

HOME = os.path.expanduser("~")
SESSIONS = os.path.join(HOME, ".claude", "sessions")


def proc_start(pid):
    return subprocess.run(["ps", "-o", "lstart=", "-p", str(pid)], capture_output=True,
                          text=True, env={**os.environ, "LC_ALL": "C", "TZ": "UTC"}).stdout.strip()


def load_token(pid):
    cands = glob.glob(os.path.join(SESSIONS, f"{pid}.*.key"))
    if not cands:
        return None
    txt = open(cands[0], "rb").read().decode("utf-8", "replace").strip()
    try:
        obj = json.loads(txt)
        if isinstance(obj, dict) and isinstance(obj.get("peerToken"), str):
            return obj["peerToken"]
    except json.JSONDecodeError:
        pass
    return txt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pid", type=int, required=True)
    ap.add_argument("--text", required=True)
    ap.add_argument("--notify", action="store_true")
    ap.add_argument("--seconds", type=int, default=20)
    args = ap.parse_args()

    me = os.getpid()
    sock_path = f"/tmp/cc-socks/{me}.sock"
    reg_path = os.path.join(SESSIONS, f"{me}.json")
    reply = f"uds:{sock_path}"

    if os.path.exists(sock_path):
        os.unlink(sock_path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(sock_path)
    os.chmod(sock_path, 0o600)
    srv.listen(8)
    srv.settimeout(1.0)

    now = int(time.time() * 1000)
    reg = {"pid": me, "sessionId": str(uuid.uuid4()), "cwd": os.getcwd(), "startedAt": now,
           "procStart": proc_start(me), "version": "2.1.302", "peerProtocol": 1,
           "peerFeatures": ["notify_idle", "reply_across_default_dirs", "artifact_yield"],
           "kind": "interactive", "entrypoint": "cli", "pidDomain": "darwin",
           "messagingSocketPath": sock_path, "name": "rc2s2-probe", "nameSource": "derived",
           "nameSince": now, "updatedAt": now, "status": "idle", "statusUpdatedAt": now}
    json.dump(reg, open(reg_path, "w"))
    print(f"probe pid={me} reply={reply}")

    captured = []

    def serve():
        t0 = time.time()
        while time.time() - t0 < args.seconds:
            try:
                conn, _ = srv.accept()
            except socket.timeout:
                continue
            conn.settimeout(3.0)
            buf = b""
            try:
                while True:
                    d = conn.recv(65536)
                    if not d:
                        break
                    buf += d
            except socket.timeout:
                pass
            conn.close()
            for line in buf.decode("utf-8", "replace").splitlines():
                if line.strip():
                    captured.append(line)
                    print("REPLY <<<", line)

    th = threading.Thread(target=serve, daemon=True)
    th.start()

    token = load_token(args.pid)
    reg_t = json.load(open(os.path.join(SESSIONS, f"{args.pid}.json")))
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(reg_t["messagingSocketPath"])
    frames = []
    if token:
        frames.append({"type": "auth", "token": token})
    frames.append({"type": "user", "message": {"role": "user", "content": args.text}, "from": reply})
    if args.notify:
        frames.append({"type": "control", "action": "notify_when_idle", "from": reply,
                       "msg_id": f"cc-msg-{os.urandom(16).hex()}"})
    for fr in frames:
        r = dict(fr)
        if r.get("type") == "auth":
            r["token"] = "<redacted>"
        print("SEND >>>", json.dumps(r))
        s.sendall((json.dumps(fr) + "\n").encode())
    s.close()

    th.join()
    print(f"captured {len(captured)} reply frame(s)")
    try:
        os.unlink(reg_path)
    except OSError:
        pass
    try:
        os.unlink(sock_path)
    except OSError:
        pass


if __name__ == "__main__":
    main()
