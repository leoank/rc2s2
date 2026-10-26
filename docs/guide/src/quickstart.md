# Quickstart on one machine

Ten minutes to a hub and a node on your own laptop, with a Claude Code session you
watch and answer from the command line. Nothing here needs the phone; the
[Setting up](setup/hub.md) section moves the same pieces to their real homes.

You need [Nix](https://nixos.org/download) with flakes enabled and
[Claude Code](https://docs.anthropic.com/en/docs/claude-code) logged in. macOS and Linux
both work.

## 1. Get the binaries

```console
$ nix profile install github:leoank/rc2s2#rc2s2 github:leoank/rc2s2#rc2s2-hub
```

Or run them without installing: `nix run github:leoank/rc2s2#rc2s2 -- --help`.
`rc2s2` is the node daemon, the terminal wrapper and a command-line client; `rc2s2-hub`
is the hub.

## 2. Make tokens

The hub reads one file of node tokens and one of client tokens, each a line of
`<name> <token>`. The node and the client each get a file holding only their token.

```console
$ mkdir -p ~/.config/rc2s2 && cd ~/.config/rc2s2
$ echo "laptop $(openssl rand -hex 24)" > node-tokens
$ echo "me $(openssl rand -hex 24)" > client-tokens
$ awk '{print $2}' node-tokens > node-token
$ awk '{print $2}' client-tokens > client-token
$ chmod 600 *
```

## 3. Start the hub

```console
$ rc2s2-hub --listen 127.0.0.1:7700 --db ~/.config/rc2s2/hub.db \
    --node-tokens ~/.config/rc2s2/node-tokens --client-tokens ~/.config/rc2s2/client-tokens
INFO rc2s2_hub: hub serving listen=127.0.0.1:7700 db=/Users/you/.config/rc2s2/hub.db
```

Leave it running and open a second terminal.

## 4. Start the node daemon

```console
$ rc2s2 node serve --hub http://127.0.0.1:7700 \
    --token-file ~/.config/rc2s2/node-token --name laptop
```

The hub logs `node registered node=laptop`. Leave this running too.

## 5. Point Claude Code at the daemon

`rc2s2 node hooks` prints a `hooks` block that sends every Claude Code event to the
daemon. It has to live in your own `~/.claude/settings.json` (a `--settings` file misses
the session-start event). If you have no hooks of your own yet, merge it in with `jq`:

```console
$ cp ~/.claude/settings.json ~/.claude/settings.json.bak
$ rc2s2 node hooks > /tmp/rc2s2-hooks.json
$ jq -s '.[0] * .[1]' ~/.claude/settings.json.bak /tmp/rc2s2-hooks.json > ~/.claude/settings.json
```

If you already have hooks, add rc2s2's entries to each event's list by hand instead:
`jq`'s `*` replaces lists rather than appending to them.

## 6. Watch a session

In a third terminal, in a project directory you have already trusted in Claude Code:

```console
$ rc2s2 run
```

This is `claude` under the daemon's wrapper. Ask it to do something that needs a
permission, such as "create a file hello.txt saying hi". While the dialog is on screen,
look at it from the command line in a fourth terminal. `ctl.toml` tells `rc2s2 ctl`
where the hub and the token are; see [rc2s2 ctl](using/ctl.md):

```console
$ printf 'hub = "http://127.0.0.1:7700"\ntoken_file = "~/.config/rc2s2/client-token"\n' \
    > ~/.config/rc2s2/ctl.toml
$ rc2s2 ctl sessions
34c8448e45f342e285d99386e9882fb5  5b0c2f1e-8a1d-4c3e-9f00-1234567890ab  pid 81234  …  /Users/you/src/app
$ rc2s2 ctl pending
34c8448e45f342e285d99386e9882fb5  acc307f5323845009aaf96c2cd927196  5b0c2f1e-…  PENDING_KIND_PERMISSION  Write  {"content":"hi","file_path":"/Users/you/src/app/hello.txt"}
$ rc2s2 ctl allow laptop acc307f5323845009aaf96c2cd927196
ok
```

The columns are the node's id, the request's id, the session's id, what kind of request
it is, the tool and its input. Commands take the node by name or id.

The dialog in the terminal disappears and Claude writes the file. Try
`rc2s2 ctl prompt laptop <session id> "now delete it"` and `rc2s2 ctl watch` to see
changes as they happen.

## Next

- Run the hub somewhere always on: [The hub](setup/hub.md).
- Run the daemon at login on every machine: [A node](setup/node.md).
- Install the phone app: [The Android app](setup/app.md).

To undo the quickstart, restore `~/.claude/settings.json.bak` and stop both processes.
