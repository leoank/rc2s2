#!/usr/bin/env python3
"""Fake cross-session messaging receiver ("rc2s2-sink").

Binds a unix socket under /tmp/cc-socks/<pid>.sock, registers a matching entry
in ~/.claude/sessions/<pid>.json shaped like a real interactive session so that
a real `claude` session's ListAgents/SendMessage can discover and reach it, and
logs every byte received. Used to capture how the real client frames and
authenticates an outbound cross-session message, and to act as a reply address
for peer_message_status / peer_idle_notice frames.

Usage: sink.py LOGFILE [--name rc2s2-sink] [--seconds 120]

Removes its registry entry and socket on exit. Never touches sockets or registry
entries it did not create.
"""
import json, os, socket, subprocess, sys, time, uuid, signal, argparse

HOME = os.path.expanduser("~")
SESSIONS = os.path.join(HOME, ".claude", "sessions")


def proc_start(pid):
    out = subprocess.run(
        ["ps", "-o", "lstart=", "-p", str(pid)],
        capture_output=True, text=True,
        env={**os.environ, "LC_ALL": "C", "TZ": "UTC"},
    ).stdout.strip()
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("logfile")
    ap.add_argument("--name", default="rc2s2-sink")
    ap.add_argument("--seconds", type=int, default=120)
    args = ap.parse_args()

    pid = os.getpid()
    sockdir = "/tmp/cc-socks"
    os.makedirs(sockdir, mode=0o700, exist_ok=True)
    sock_path = os.path.join(sockdir, f"{pid}.sock")
    reg_path = os.path.join(SESSIONS, f"{pid}.json")

    if os.path.exists(sock_path):
        os.unlink(sock_path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(sock_path)
    os.chmod(sock_path, 0o600)
    srv.listen(8)
    srv.settimeout(1.0)

    now_ms = int(time.time() * 1000)
    reg = {
        "pid": pid,
        "sessionId": str(uuid.uuid4()),
        "cwd": os.getcwd(),
        "startedAt": now_ms,
        "procStart": proc_start(pid),
        "version": "2.1.302",
        "peerProtocol": 1,
        "peerFeatures": ["notify_idle", "reply_across_default_dirs", "artifact_yield"],
        "kind": "interactive",
        "entrypoint": "cli",
        "pidDomain": "darwin",
        "messagingSocketPath": sock_path,
        "name": args.name,
        "nameSource": "derived",
        "nameSince": now_ms,
        "updatedAt": now_ms,
        "status": "idle",
        "statusUpdatedAt": now_ms,
    }
    os.makedirs(SESSIONS, exist_ok=True)
    with open(reg_path, "w") as f:
        json.dump(reg, f)

    log = open(args.logfile, "ab")

    def logline(s):
        log.write(f"\n### sink {time.strftime('%H:%M:%S')}: {s}\n".encode())
        log.flush()

    logline(f"listening pid={pid} name={args.name} sock={sock_path} sessionId={reg['sessionId']}")

    def cleanup(*_):
        logline("shutting down; removing registry entry and socket")
        try:
            os.unlink(reg_path)
        except OSError:
            pass
        try:
            os.unlink(sock_path)
        except OSError:
            pass
        log.close()
        sys.exit(0)

    signal.signal(signal.SIGTERM, cleanup)
    signal.signal(signal.SIGINT, cleanup)

    t0 = time.time()
    conn_n = 0
    while time.time() - t0 < args.seconds:
        try:
            conn, _ = srv.accept()
        except socket.timeout:
            continue
        conn_n += 1
        cid = conn_n
        logline(f"connection #{cid} opened")
        conn.settimeout(5.0)
        try:
            while True:
                data = conn.recv(65536)
                if not data:
                    break
                log.write(f"\n### sink {time.strftime('%H:%M:%S')}: conn#{cid} recv {len(data)} bytes >>>\n".encode())
                log.write(data)
                log.write(b"\n<<<\n")
                log.flush()
        except socket.timeout:
            logline(f"connection #{cid} recv timeout")
        except OSError as e:
            logline(f"connection #{cid} error {e}")
        finally:
            conn.close()
            logline(f"connection #{cid} closed")
    cleanup()


if __name__ == "__main__":
    main()
