---
id: RUN-20261017-dssgwx
previous_id: manthan/RUN-20261017-syvm
type: run
schema_version: 1
kind: compute               # bench | compute | assembly | calibration
project: PRJ-0001
experiment: EXP-20261016-rq1407
protocols: []
status: done           # planned | in_progress | done | aborted
created: 2026-10-17
author: agent:claude
started: 2026-10-17T13:34:00+01:00
finished: 2026-10-17T13:52:00+01:00
commit: cc33196
---

## Notes
The untested item of [[EXP-20261016-rq1407]]: can a hook reach the host over an AF_UNIX socket while
loopback TCP is denied (`hostLoopback: deny`), so a loopback egress proxy and a hook
endpoint can coexist on seatbelt? Spike binary rebuilt from `spikes/07-mxc/` in the
`rc2s2/core` worktree (`nix develop -c cargo build`); Claude Code 2.1.302 (wrapper
`/nix/store/yp8ar…-claude-code/bin/claude`). Files in `experiments/EXP-20261016-rq1407/run-unix-hook/`;
the listener and the claude policy also in `spikes/07-mxc/` (`unix_hook_server.py`,
`policy-claude-unix-hook.json`).

**Setup.** Scratch under `/private/tmp/rc2s2-sbx-unixhook/` (`config`, `tmp`, `work`,
`hooks`; the session scratchpad path would put the socket at 124 bytes, over the 104-byte
`sun_path` limit). Host side: `unix_hook_server.py RUNDIR hooks/hook.sock 2`, an
`http.server` handler on `socketserver.UnixStreamServer` that appends every event to
`events.jsonl`, answers `Stop` with `{}` and holds `PermissionRequest` 2 s (or until
`answer.json` appears) before returning `{"hookSpecificOutput": {"hookEventName":
"PermissionRequest", "decision": {"behavior": "allow"}}}`. A second listener,
`python3 -m http.server 18765 --bind 127.0.0.1`, is the negative control. HTTP hooks only
speak TCP, so the sandbox side is a command hook:

```
/usr/bin/curl -s --max-time 55 --unix-socket /private/tmp/rc2s2-sbx-unixhook/hooks/hook.sock \
  -X POST -H 'Content-Type: application/json' --data-binary @- http://hook/hook
```

on both `Stop` and `PermissionRequest` in `<config>/settings.json` (`settings-unix-hook.json`,
hook `timeout: 60`). macOS `/usr/bin/curl` 8.7.1 has `--unix-socket`; the host part of the
URL is ignored.

**Policy** (`policy-claude-unix-hook.json`) = `policy-claude-min.json` plus: the hooks dir
read-write; the work dir read-write instead of read-only (claude has to write
`hello.txt`); `ingress: deny` and `hostLoopback: deny` written out (the min policy omitted
them, which the SDK already resolves to deny, so the network rules are identical: the
profile ends with `(deny network-outbound (remote ip "localhost:*"))`). Egress stays
`allow` so claude reaches the API without a proxy; the spike's policy shape has no proxy
field.

**Plain-shell probes** (`test-sh-unix-hook.log`; the `exit=` lines after an `echo` are a tee
artefact, the `mxc-spike: exited N` line is the result):

| policy | TCP 127.0.0.1:18765 | AF_UNIX hooks/hook.sock |
|---|---|---|
| egress allow, hostLoopback deny, hooks rw (`policy-sh-unix-hook.json`) | refused (curl 7) | `{}`, exit 0 |
| same, `localhost:18765` | refused | - |
| egress deny, hooks rw (`…-egress-deny.json`) | refused | `{}`, exit 0 |
| egress allow, hooks read-only (`…-readonly.json`) | - | refused (curl 7) |
| egress allow, hooks in `denied` (`policy-sh-denied.json`) | - | refused |
| egress allow, hooks not granted at all (`policy-sh-no-grant.json`) | - | `{}`, exit 0 |
| egress deny, hooks not granted (`…-no-grant-egress-deny.json`) | - | refused |

A 3-line JSON piped through `sh -c '… | curl --data-binary @-'` arrived intact (59 bytes,
parsed). So the AF_UNIX side matches the profile builder (`profile_builder.rs`): a
read-write path is emitted as `(allow file-read* file-write* network-bind
network-outbound (subpath …))`, a read-only path adds `(deny file-write* network-bind
network-outbound (subpath …))`, a denied path denies all four. Under `egress: allow`
the unfiltered `(allow network-outbound)` also covers AF_UNIX, so the sandbox can connect
to any unix socket on the host that is not under a read-only or denied subpath (other
sessions' `/tmp/cc-socks/<pid>.sock`, launchd agent sockets, a Docker socket); the
`hostLoopback: deny` rule is IP-only and does not touch this. Under `egress: deny` only
read-write subpaths connect. The EXP-20261016-rq1407 line "AF_UNIX connect allowed under a read-write
path" is therefore the egress-deny case; under egress-allow it is "everywhere except
read-only and denied paths".

**claude -p, run 1** (`test-claude-1-stop.log`): `claude -p "Reply with just ok." --model
sonnet --output-format json`: `ok`, exit 0, 6.05 s wall (`duration_api_ms` 3169). The `Stop`
event (475 bytes, full JSON with `session_id`, `transcript_path`, `cwd`, `prompt_id`,
`permission_mode`, `last_assistant_message`) arrived on the unix socket while loopback TCP
was denied. Credential leased from the keychain into `<config>/.credentials.json` (mode
600, written straight from `security … -w`, never displayed) after 13:45, because an
earlier lease experiment had rotated the token set.

**claude -p, run 2, permission round trip** (`test-claude-2-permission.log`): prompt
"Create a file named hello.txt containing hi in the current directory. Do not ask
questions." with `--permission-mode default`. Timeline from `events.jsonl` (launch at
13:49:16.30):

- +2.95 s `PermissionRequest` in, `tool_name: Write`, `tool_input` with `file_path` and
  `content: "hi\n"`, `permission_suggestions: [{setMode acceptEdits}]` (596 bytes);
- held 2.05 s on the host, answered allow;
- +1.36 s after the answer `Stop` in (577 bytes, `last_assistant_message` "I created
  `hello.txt` …");
- process exit at 6.19 s wall, exit 0, `duration_api_ms` 2718; `work/hello.txt` = `hi`.

So the hold is honoured end to end over AF_UNIX: claude blocks on the command hook, the host
decides, the tool runs. The hook command's cwd was the work dir (`cwd` in the event); the
hook used absolute paths so it did not matter. The transcript landed in
`<config>/code/-private-tmp-rc2s2-sbx-unixhook-work/<session>.jsonl`, readable outside.

**Consequence for the architecture.** Proxy-only egress on seatbelt (`egress: deny` + a
loopback proxy, which the SDK only accepts with `hostLoopback: deny`) and a hook endpoint
do coexist: the hook goes over a unix socket in a read-write directory (the egress-deny
row above is exactly that filesystem rule; the proxy's port-scoped IP allow is additive and
was not exercised here because the spike has no proxy mapping). Under `egress: allow` the
hooks dir needs no grant at all, but the sandbox can then reach every host unix socket, so
known control sockets should be listed in `denied` or egress kept at deny.

Cleanup: both listeners killed; `/private/tmp/rc2s2-sbx-unixhook/` removed with the leased
credential (verified absent); no `Claude Code-credentials-b0c06bf7` keychain item (the
suffix for this config dir) appeared, the file lease was never rotated.

## Checklist
- [x] loopback TCP refused from inside under hostLoopback deny (127.0.0.1 and localhost)
- [x] AF_UNIX POST from inside reaches the host listener under the same policy
- [x] AF_UNIX rule characterised: rw allow, ro and denied refuse, egress allow opens all other paths, egress deny closes them
- [x] claude -p turn inside the sandbox with the Stop hook over the unix socket
- [x] PermissionRequest held 2 s on the host, allow applied, file created, Stop follows
- [x] leased config dir deleted, no suffixed keychain item left
