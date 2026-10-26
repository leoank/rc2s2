# rc2s2

Watch and steer your Claude Code sessions from your phone. A small daemon on every
machine reports each session to one hub; an Android app talks to the hub. When Claude
stops for a permission or a question, your phone buzzes and you answer from the
notification.

**Documentation: <https://leoank.github.io/rc2s2/>**

- Every Claude Code session on every machine, listed live with its conversation, including
  sessions started by hand in a terminal.
- Allow or deny permission dialogs and answer Claude's questions from the phone; ignore
  them and the terminal's dialog works as usual.
- Prompt any session; interrupt and kill sessions started with `rc2s2 run`.
- Notifications through UnifiedPush and ntfy, including alerts when your Claude rate
  limits are running out early.
- Node health and rate-limit usage as charts per machine.

rc2s2 is in active development, built for one person's own machines on one Tailscale
network. Claude Code is the only agent and Android the only client so far; starting
sessions from the phone and sandboxed sessions are planned.

## Try it

With [Nix](https://nixos.org/download) (flakes enabled):

```console
$ nix profile install github:leoank/rc2s2#rc2s2 github:leoank/rc2s2#rc2s2-hub
```

then follow the [quickstart](https://leoank.github.io/rc2s2/quickstart.html) to run a hub
and a node on one machine, and [Setting up](https://leoank.github.io/rc2s2/setup/hub.html)
for a real deployment and the Android app.

## Repository

| Path | What |
|---|---|
| `crates/node` | `rc2s2`: the node daemon, the `rc2s2 run` wrapper and `rc2s2 ctl` |
| `crates/hub` | `rc2s2-hub`: gRPC services, SQLite, notifications |
| `crates/proto` | The protobuf definitions of both services |
| `crates/core` | The client core in Rust, exposed to Kotlin through UniFFI |
| `android/` | The Compose app |
| `docs/guide/` | The user guide (mdBook) |
| `docs/architecture.md`, `docs/decisions/` | The design and one record per decision |
| `notes/` | The project's [yajna](https://github.com/leoank/yajna) lab notebook: experiments, runs, progress |

## Development

```console
$ nix develop                         # Rust toolchain and protoc
$ cargo test --workspace
$ nix flake check                     # the workspace built and tested, the guide built
$ nix build .#docs                    # the guide, into result/
$ nix develop .#android               # Android SDK, NDK, Gradle (several GB)
$ cd android && gradle assembleDebug
```

Agents picking this up: start with `docs/architecture.md` and the decision records in
`docs/decisions/`, then the *Progress* checklist on the notebook page, `notes/README.md`,
for what has been built. `docs/HANDOFF.md` is the phase 0 handoff, kept for its history.
