#!/usr/bin/env python3
"""External same-user daemon that delivers a prompt to a running Claude Code
session over its per-session unix messaging socket.

Reads ~/.claude/sessions/<pid>.json for the target socket path, optionally reads
the sibling <pid>.<64hex>.key auth key, connects to the socket, and sends a
newline-delimited JSON (NDJSON) auth line followed by a user-message line, as the
`[uds-messaging]` server expects. Optionally sends a notify_when_idle control
frame and listens briefly for a peer_message_status / peer_idle_notice reply.

The auth key value is never printed; only its byte length / shape is reported.

Usage:
  daemon.py --pid PID --text "..." [--no-auth] [--from-mode bypass|prompting]
            [--reply uds:/path] [--notify] [--session-id UUID] [--priority now|next|later]
            [--listen SECONDS]
"""
import argparse, json, os, socket, sys, time, glob

HOME = os.path.expanduser("~")
SESSIONS = os.path.join(HOME, ".claude", "sessions")


def find_key(pid):
    cands = glob.glob(os.path.join(SESSIONS, f"{pid}.*.key"))
    return cands[0] if cands else None


def load_token(pid):
    """Return (token_string_or_None, shape_description). Never returns/prints raw bytes elsewhere."""
    kf = find_key(pid)
    if not kf:
        return None, "no key file present"
    with open(kf, "rb") as f:
        raw = f.read()
    shape = f"keyfile basename=<{len(os.path.basename(kf))} chars> bytes={len(raw)}"
    txt = raw.decode("utf-8", "replace").strip()
    # key file may be raw token or JSON {"token": "..."}
    token = txt
    try:
        obj = json.loads(txt)
        if isinstance(obj, dict):
            for k in ("token", "peerToken", "key", "secret"):
                if isinstance(obj.get(k), str):
                    token = obj[k]
                    shape += f" json:{sorted(obj.keys())}"
                    break
    except json.JSONDecodeError:
        shape += " (raw, not json)"
    return token, shape


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pid", type=int, required=True)
    ap.add_argument("--text", required=True)
    ap.add_argument("--no-auth", action="store_true")
    ap.add_argument("--from-mode", choices=["bypass", "prompting"], default=None)
    ap.add_argument("--reply", default=None, help="reply address, e.g. uds:/tmp/cc-socks/NNN.sock")
    ap.add_argument("--notify", action="store_true")
    ap.add_argument("--session-id", default=None)
    ap.add_argument("--priority", choices=["now", "next", "later"], default=None)
    ap.add_argument("--listen", type=float, default=2.0)
    args = ap.parse_args()

    reg_path = os.path.join(SESSIONS, f"{args.pid}.json")
    with open(reg_path) as f:
        reg = json.load(f)
    sock_path = reg["messagingSocketPath"]
    print(f"target pid={args.pid} sessionId={reg.get('sessionId')} status={reg.get('status')} sock={sock_path}")

    token, shape = (None, "auth disabled (--no-auth)")
    if not args.no_auth:
        token, shape = load_token(args.pid)
    print(f"auth: {shape}; sending_auth={token is not None}")

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(sock_path)

    lines = []
    if token is not None:
        lines.append({"type": "auth", "token": token})

    user = {"type": "user", "message": {"role": "user", "content": args.text}}
    if args.session_id:
        user["session_id"] = args.session_id
    if args.priority:
        user["priority"] = args.priority
    if args.reply:
        user["from"] = args.reply
    if args.from_mode:
        user["from_mode"] = args.from_mode
    lines.append(user)

    if args.notify and args.reply:
        notif = {"type": "control", "action": "notify_when_idle",
                 "from": args.reply, "msg_id": f"cc-msg-{os.urandom(16).hex()}"}
        if args.from_mode:
            notif["from_mode"] = args.from_mode
        lines.append(notif)

    for ln in lines:
        redacted = dict(ln)
        if redacted.get("type") == "auth":
            redacted["token"] = "<redacted>"
        print("send:", json.dumps(redacted))
        s.sendall((json.dumps(ln) + "\n").encode())

    # listen for any reply (peer_message_status / peer_idle_notice) on this same conn
    s.settimeout(args.listen)
    try:
        while True:
            data = s.recv(65536)
            if not data:
                break
            print("recv:", data.decode("utf-8", "replace").rstrip())
    except socket.timeout:
        pass
    s.close()
    print("done")


if __name__ == "__main__":
    main()
