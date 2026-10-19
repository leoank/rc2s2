# Claude Code cross-session messaging socket — protocol

Reverse-engineered from the printable strings and plain-JS regions of
Claude Code 2.1.302 (`.claude-wrapped`, a bun executable). Identifiers below are
the minified names as they appear in the bundle; the behavioural strings are
quoted verbatim. No secrets, tokens or key-file contents are reproduced here.

Source regions (in the `cli/` strings dump): the `[uds-messaging]` server and
transport live in `msg-9.txt` (module `chunk-*` exporting `_8s`/`mn`, `nn`,
`be`, `ze`); the inbound permission gate and idle module in `msg-9.txt`
(`chunk-tbtk3zbk`, `chunk-vsj75eyd`); the address/envelope helpers in
`msg-4.txt` (`chunk-1fschhqb`); the server-state class in `msg-8.txt`
(`chunk-jqxbhf1h`, class `t`); the string table in `msg-0.txt`; the SendMessage
tool in `msg-10.txt`.

## 1. Where the socket and registry live

- Socket path (auto): `${XDG_RUNTIME_DIR||tmpdir}/cc-socks/<pid>.sock`, resolved;
  on macOS `tmpdir` is `/tmp` → `/tmp/cc-socks/<pid>.sock`. Max path length
  **103 bytes** (`Q=103`). Per-uid fallback `…/cc-socks-<uid>/<pid>.sock` when the
  primary dir is refused. Overridable with `--messaging-socket-path <abs path>`.
- The `cc-socks` directory must be mode `0700` and owned by you or root (long
  vetting walk in `Re()`: refuses symlinks, world/group-writable-without-sticky,
  foreign owner, uid-collapse in unmapped user namespaces). The socket file itself
  is `chmod 0600` (`await Ae(e,384)` → octal 600), so only the same uid can connect.
- Registry record: `~/.claude/sessions/<pid>.json`, world-readable (`0644`).
  Fields (from live records):
  `pid, sessionId, cwd, startedAt(ms), procStart (LC_ALL=C TZ=UTC ps -o lstart=),
  version, peerProtocol(=1), peerFeatures(["notify_idle","reply_across_default_dirs","artifact_yield"]),
  kind("interactive"), entrypoint, pidDomain("darwin"), messagingSocketPath,
  name, nameSource, nameSince, updatedAt, status("idle"|"busy"), statusUpdatedAt`.
- Auth key: sibling file `~/.claude/sessions/<pid>.<64 hex>.key`, mode `0600`,
  108 bytes on this machine. Published at listen time by `Uzs(sockpath, peerToken, …)`;
  deleted on exit (`DJo` on `process.exit`). Name regex `^(\d+)\.[0-9a-f]{64}\.key$`.

## 2. Transport framing

Newline-delimited JSON (**NDJSON**), UTF-8, one JSON object per line
(`e.setEncoding("utf8")`; the `data` handler splits on `\n`, `JSON.parse`s each
non-blank line, and parses a trailing fragment on `end`). Properties:

- **Line length cap** `krn` chars; exceeding it drops the whole connection
  (`"Line exceeded … chars; dropping connection"`).
- **First-line deadline** `firstLineDeadlineMs`, default `rsr = 30000` ms: a
  connection that sends no complete line within 30 s is closed
  (`"Closing a connection that sent no complete line within … ms"`).
- Blank lines are skipped (but a blank line before auth, when auth is required,
  counts as a failed auth and drops the connection).
- The server's own startup log states the wire format literally:
  `Inject messages (auth line REQUIRED|optional here): { echo '{"type":"auth","token":"'"$CLAUDE_CODE_MESSAGING_TOKEN"'"}'; echo '{"type":"user","message":{"role":"user","content":"hello"}}'; } | socat - UNIX-CONNECT:<path>`

## 3. Message types (top-level `type` field)

Dispatch in `be()`; a message with no string `type` is ignored
(`"Ignoring message without valid type field"`).

### `auth` (first line, optional-or-required)
`{"type":"auth","token":"<token>"}`. Validated by `zzs(token, activeTokens)`.
`activeTokens = $zs()` yields a pair: a **peerToken** (published to the `.key`
file, for external same-user peers) and a **childToken** (exported to child
processes as env `CLAUDE_CODE_MESSAGING_TOKEN`). The validator returns a verdict
(the token *class*, "peer" vs "child") that flows downstream as the 5th handler
arg; the child class is what lets a self-spawned child be treated as `selfSent`.

### `user` (a prompt delivered to the session) — the one that matters for rc2s2
`{"type":"user","message":{"role":"user","content":"<string>"}}` plus optional
`session_id, uuid, priority("now"|"next"|"later"), from, from_mode, from_plugin,
msg_id, file_attachments`.

Handler `ze()`:
1. `message.content` must be a non-empty **string** or it is ignored
   (`"Ignoring user message with missing or non-string content"`).
2. If `session_id` is present it must equal the receiver's current `sessionId`
   (`q()`), else dropped (`"Dropping … message: session_id mismatch (got … expected …)"`).
   Omitting `session_id` skips the check.
3. Content is wrapped in a `<cross-session-message from="…">…</cross-session-message>`
   envelope (see §6), `file_attachments` are materialized (`materializeLocalPeerFiles`),
   then an origin object is built:
   `{kind:"peer", from, verifiedPeerPid, verifiedPeerProcStart, selfSent, msg_id, plugin, …}`.
4. The message is passed through the **inbound gate** `HVt(R)` (§5). If the gate
   does not return `"accept"` the message is **not enqueued** here (it was held or
   refused inside the gate, which already sent the sender a receipt). On `"accept"`
   it is enqueued with `ET(R)` as `{mode:"prompt", isMeta:true, skipSlashCommands:true,
   skipAttachments:true, priority, origin}` — i.e. it becomes a **user turn** that
   drains at the session's next tool round / when it next reads the queue.

### `control` (an action frame) — `{"type":"control","action":"…", …}`
Actions handled in `be()` (all first pass `session_id` check `Ie()`):
- `rename` `{name}` — renames the session.
- `peer_message_status` `{status, orig_msg_id, reason?, drop_reason?, dropped_msg_ids?}`
  with `status ∈ {held,denied,expired,delivered,refused,dropped}`. Sent by a
  *receiver back to a sender*; correlates to an outstanding send and surfaces to
  the sender as a `[Cross-session delivery notice]`.
- `notify_when_idle` `{from:"uds:<reply-addr>", msg_id, from_mode?}` (zod schema
  `D9r`). Subscribes the sender to one idle/exit callback. Reply target must be a
  shaped address inside the cc-socks namespace and not self.
- `peer_idle_notice` `{orig_msg_id, state, finished_at?, detail?, from?, from_mode?}`
  (schema `L9r`) — the callback for the above; `state ∈ {idle,exited,unavailable}`.
- `yield_artifact_replies` / `unyield_artifact_replies` / `artifact_replies_yielded`
  — artifact comment-reply handoff between sessions of one conversation (not
  relevant to prompt delivery).
Unknown actions/types are logged and ignored.

## 4. Peer identity verification

On each connection the server reads the kernel-verified peer credentials:
`Hpt(socket)` → **peer pid** (getsockopt `LOCAL_PEERPID` on macOS / `SO_PEERCRED`
on Linux; `pidDomain:"darwin"`). `Dyt(pid)` reads that pid's **process-start
token** (its `procStart`). `TNo(pid)` is consulted when a child token was
presented. These become `verifiedPeerPid` / `verifiedPeerProcStart`, which are
cross-checked against the registry record (`Rrn`/`Prn`) when a reply address or
idle subscription names a pid. Ancestry/self-send is decided in `ye()`/`Ke()`:
on macOS a peer is `selfSent` if its pid's start-token verdict is `self`
(the receiver appears in the peer's ancestor chain) or `no-evidence` + a child
token was presented. A same-user external daemon that is **not** an ancestor and
presents the peerToken is therefore a verified peer, `selfSent=false`.

## 5. The inbound permission gate ("permission-mode parity")

`chunk-tbtk3zbk`. A per-message decision `accept` / `hold` / `refuse`:

- **Kill switch**: if cross-session messaging is disabled (`Yi()` false) → `refuse`
  (cause `kill-switch`).
- **Explicit setting** `crossSessionInbound` (`accept`|`hold`|`refuse`), read with
  precedence policy/flag/user > repo (takes the stricter), wins if set.
- **Otherwise mode-based** (`E(e,false)` for a peer message):
  - `selfSent` → `accept`.
  - Let `i = rkt(currentMode)` = `"bypass"` if the receiver is in
    `bypassPermissions` (i.e. started with `--dangerously-skip-permissions`),
    else `"prompting"` (default / plan / acceptEdits).
  - The sender's asserted `from_mode` is only consulted when feature flag
    `tengu_harbor_kite_mode_emit` (`tkt()`) is on; then equal mode → accept,
    mismatch → `hold("mode-mismatch")`.
  - With the flag off (the common case) the decision is purely the **receiver's**
    mode: **`i==="bypass"` → `hold`**, **`"prompting"` → `accept`**.

**Consequence (verified dynamically, §RUN):** a message from an external daemon
is delivered straight to the model when the receiving session runs in the
**default (prompting) permission mode**, and is **held for the user's approval**
when the receiver runs in **bypass** mode. The receipt the sender gets for a held
message is: *"Your message is held for the recipient user's approval before it
reaches their Claude session (permission-mode parity)."*

Held messages sit in a buffer (cap `Q=100`, oldest evicted as `expired`). The
receiving user approves/denies them (`ikt(id,"approve"|"deny")`), which releases
to the queue or drops with a receipt. Refuse/kill-switch send immediate receipts.

## 6. How a delivered message appears to the model

Enqueued as a user-role turn, `isMeta:true`, with the body wrapped:

```
<cross-session-message from="<reply-addr>" from-session="…" from-name="…" from-mode="…" from-plugin="…">
<your text>
</cross-session-message>
```

(envelope built by `V()`/`Iyt()` in `chunk-1fschhqb`; `from` is the only always-
present attribute). SendMessage's own tool prompt documents the same: *"Your
message arrives wrapped as `<cross-session-message from=…>`. To reply … copy its
`from` attribute as your `to`."* and *"messages enqueue and drain at the
receiver's next tool round."* The model reads the text **literally** (an `@path`
in it attaches nothing).

## 7. What a same-user external daemon needs

1. Read `~/.claude/sessions/<pid>.json` → `messagingSocketPath` (and check `pid`
   is alive + `status`).
2. `connect(AF_UNIX, sock_path)` — allowed because same uid (socket is 0600).
3. If the session requires auth, read the sibling `<pid>.<64hex>.key` (same uid,
   0600) and send `{"type":"auth","token":<file contents>}` as the first line.
4. Send `{"type":"user","message":{"role":"user","content":"…"}}`. Optionally set
   `session_id` (must match), `priority`, `from` (a `uds:` reply address for
   receipts/idle), `from_mode`.
5. The kernel supplies the daemon's real pid as `verifiedPeerPid`; the daemon is a
   verified same-user peer but not `selfSent`.
6. Gate: delivered immediately if the receiver is in a prompting mode; held for
   approval if the receiver is in bypass mode.

No signing beyond the pre-shared `.key` token is required; identity rests on the
`0700` dir + `0600` socket/key (same-uid only) plus kernel `SO_PEERCRED`.

## 8. Limitations for rc2s2

- There is **no "interrupt current turn" message type**. `user` frames with
  `priority:"now"` jump the queue but still drain at a tool boundary; nothing in
  the protocol cancels an in-flight turn. Interrupting a busy session (Esc/Ctrl-C)
  still needs PTY/keystroke injection.
- A bypass-mode session holds daemon prompts for user approval, so for unattended
  delivery the session should run in a prompting mode (or set
  `crossSessionInbound: accept`).
- Delivery is one-way enqueue; to observe the model's reply you either watch the
  transcript/PTY or register a `uds:` reply address and read `peer_message_status`
  / `peer_idle_notice`.
- Socket + key are per-pid and deleted on exit; a daemon must re-resolve them from
  the registry each time and must verify `pid`/`procStart` (entries outlive their
  process).
