# How it works

rc2s2 has three parts. You run one hub, one node daemon on every machine where Claude
Code runs, and the app on your phone.

```text
 phone                       hub machine                  each node machine
┌──────────────┐  gRPC     ┌──────────────┐   gRPC      ┌───────────────────┐
│ Android app  │◄─────────►│ rc2s2-hub    │◄───────────►│ rc2s2 node serve  │
└──────▲───────┘           │  SQLite      │  (outbound  │   ▲ hooks (HTTP)  │
       │ push              │  notifier    │   from the  │   │ transcripts   │
       └───────────────────┤              │   node)     │ claude  claude    │
         (ntfy/UnifiedPush)└──────────────┘             └───────────────────┘
```

## The node daemon

`rc2s2 node serve` runs as you, on each machine, and learns about Claude Code sessions in
three ways, none of which involve reading the terminal:

- **Hooks.** Claude Code can call an HTTP endpoint on events such as a session starting,
  a tool about to run or a permission dialog opening. You add a `hooks` block to
  `~/.claude/settings.json` that points every event at the daemon, so every session on
  the machine reports to it, including ones you start by hand.
- **Transcripts.** Each session writes its conversation to a JSONL file under
  `~/.claude/projects/`. The daemon tails it to show Claude's text and thinking, which
  hooks do not carry.
- **The session registry.** Claude Code lists its interactive sessions under
  `~/.claude/sessions/`; the daemon reads it to find sessions that started before it did.

When a permission dialog opens, the hook *waits* for the daemon's answer. The daemon
holds it while the hub asks you. If you answer on the phone, Claude proceeds. If you
press a key in the terminal, or nobody answers in time, the daemon lets go and the dialog
in the terminal works as normal.

The daemon also samples the machine's health every 30 seconds and, if you turn it on,
your Claude rate-limit usage every five minutes.

## The hub

`rc2s2-hub` is one binary with a SQLite database. Every node holds one outbound stream to
it, so nodes need no open ports. The hub records every session, every event and every
health sample, relays your decisions back to the right node, and sends notifications.
It keeps working if a node or the app disconnects; a node buffers its events until the
hub acknowledges them, so a hub restart loses nothing.

Nodes and clients prove who they are with pre-shared tokens, one per node and one per
client, listed in two files on the hub.

## The app

The Android app connects to the hub with an address and a client token. It shows every
node with its live sessions, the requests waiting for you, each session's conversation
as it happens, and each node's health as charts. Notifications come through
[UnifiedPush](https://unifiedpush.org) with the ntfy app as the distributor, so no
Google services are involved.

## Wrapped and unwrapped sessions

Every session is seen, and its dialogs and questions can be answered from the phone.
What else you can do depends on how it was started:

| | `claude` | `rc2s2 run` |
|---|---|---|
| Listed, conversation streamed | yes | yes |
| Permission dialogs and questions from the phone | yes | yes |
| Send a prompt | yes, as a message from another session | yes, typed into the terminal |
| Interrupt the current turn | no | yes |
| Kill the session | no | yes |
| A key press in the terminal releases a held dialog | no, it waits up to 2 minutes | yes, at once |

`rc2s2 run` starts `claude` under a terminal the daemon owns and mirrors it to yours, so
it looks and behaves like plain `claude`. Use it for sessions you may want to steer from
the phone. See [Sessions](using/sessions.md).

## The network

In this version every part is expected to be on the same Tailscale network: the hub
listens on a tailnet address, nodes reach it by name, and the phone runs the Tailscale
app. The hub speaks plain gRPC without TLS, so keep its port off the public internet;
Tailscale encrypts the traffic.
