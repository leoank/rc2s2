# rc2s2

rc2s2 lets you watch and steer the Claude Code sessions running on your machines from
your phone. A small daemon on each machine reports every session to one hub; an Android
app talks to the hub. When Claude stops to ask for a permission or a question, your phone
buzzes, and you can answer from the notification without opening a laptop.

```text
 phone ──────── hub ──────── node: laptop      claude, claude, ...
 (Android app)  (rc2s2-hub)  node: desktop     claude
                             node: GPU box     claude, claude
```

## What it does today

- **Every session, live.** Every Claude Code session on every machine running the
  daemon, listed by machine, with its conversation streaming as it is written: your
  prompts, Claude's replies and thinking, every tool call with its output and diff.
  Sessions you start by hand in a terminal are included; nothing has to be launched
  through rc2s2 to be seen.
- **Decide from anywhere.** A permission dialog or a question from Claude shows up on the
  phone. Allow, deny with a reason, or answer; the terminal moves on as if you had typed
  it there. Ignore it and the dialog stays in the terminal as usual.
- **Prompt, interrupt, kill.** Send a prompt to any session. Sessions started with
  `rc2s2 run` can also be interrupted and killed.
- **Notifications.** A push when a session waits on you, finishes a turn or ends, and
  when your Claude rate limits are running out early. Each kind can be turned off.
- **Node health.** CPU, memory, GPU, disk, load and your Claude rate-limit windows
  (5-hour, 7-day and the 7-day Fable limit) as charts per machine.
- **Usage per session.** Tokens, time and estimated cost per turn and per session.

## What it is not (yet)

rc2s2 is in active development and built for one person and their own machines.

- Claude Code is the only agent it understands, and Android the only client.
- Everything is expected to sit on one [Tailscale](https://tailscale.com) network
  (tailnet); the hub has no TLS and no accounts, only pre-shared tokens.
- Starting new sessions from the phone, several Claude accounts per machine and sandboxed
  sessions are planned, not built. See *Phases* in the [architecture](design/architecture.md).

## Where to go next

- [How it works](how-it-works.md) explains the three parts and what flows between them.
- [Quickstart on one machine](quickstart.md) runs a hub and a node on your laptop and
  drives a session from the command line in ten minutes.
- [Setting up](setup/hub.md) covers a real deployment: the hub on an always-on Linux
  machine, a node per machine, the app and notifications.
- The [Design](design/architecture.md) section has the architecture and the decision
  record behind every choice.
