# A node

A node is any machine where you run Claude Code: a laptop, a desktop, a build server.
Each runs `rc2s2 node serve` as the user who runs Claude Code, since it reads that
user's `~/.claude`. macOS and Linux are supported.

## 1. A token

Add a line for the machine to the hub's node-token file and restart the hub:

```text
laptop 3f9a…
```

Put the token alone in a file on the machine, readable only by you, for example
`~/.config/rc2s2/node-token`.

## 2. The daemon

```console
$ rc2s2 node serve --hub http://hub:7700 --token-file ~/.config/rc2s2/node-token --name laptop
```

`--name` is what the app shows; it defaults to the hostname. The daemon connects out to
the hub and reconnects by itself, with backoff, when the link drops. While the hub is
unreachable it keeps up to 200,000 events and sends them when the link is back.

Useful flags (all of them are in the [command line reference](../reference/cli.md)):

| Flag | What it does |
|---|---|
| `--claude-usage` | Report your Claude rate-limit usage as health, polled every five minutes. See [Rate limits](../using/health.md#rate-limits). Off by default. |
| `--keep-awake` | Keep the machine from idle-sleeping while any session is busy (macOS, `caffeinate -i`), so a turn finishes when the screen turns off. Lid-close sleep still happens. Off by default. |
| `--ignore-cwd <glob>` | Do not track sessions whose working directory matches, for example throwaway test sessions. Repeatable. |
| `--hold-secs` | How long a permission dialog of a plain `claude` session waits for the phone before falling through to the terminal. Default 120. |
| `--wrapped-hold-secs` | The same for sessions under `rc2s2 run`, where a key press releases it at once. Default 540. |
| `--hook-addr` | Where the hook endpoint listens. Default `127.0.0.1:7777`; must match the hooks block. |
| `--claude-dir` | Claude Code's config directory, if not `~/.claude` (honours `CLAUDE_CONFIG_DIR`). |

Without `--hub`, the daemon prints everything it observes to stdout as JSON lines, which
is a handy way to see what it sees.

## 3. The hooks

Claude Code has to be told to send its events to the daemon. `rc2s2 node hooks` prints
the block to merge into `~/.claude/settings.json`; it registers one HTTP hook per event
(`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`,
`Notification`, `Stop`, `SubagentStop`, `SessionEnd`), all pointing at
`http://127.0.0.1:7777/hook`.

```console
$ rc2s2 node hooks                     # print the block
$ rc2s2 node hooks --hook-addr 127.0.0.1:7788 --timeout 600
```

The block must be in your user settings file. Claude Code reads hooks passed with
`--settings` only after the session has started, so the session-start event would be
missed. Merge it by hand, or with `jq` as in the [Quickstart](../quickstart.md#5-point-claude-code-at-the-daemon)
when you have no hooks of your own. Restart sessions that were already running so they
load the hooks; until then the daemon still finds them through the session registry and
their transcripts, but cannot hold their dialogs.

If the daemon is not running, the hook calls fail and Claude Code carries on without
them: a failed HTTP hook does not block a session.

## 4. Start at login

### macOS: a launchd agent

`~/Library/LaunchAgents/me.leoank.rc2s2-node.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>me.leoank.rc2s2-node</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Users/you/.nix-profile/bin/rc2s2</string>
    <string>node</string><string>serve</string>
    <string>--hub</string><string>http://hub:7700</string>
    <string>--token-file</string><string>/Users/you/.config/rc2s2/node-token</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardErrorPath</key><string>/tmp/rc2s2-node.err.log</string>
</dict>
</plist>
```

```console
$ launchctl load ~/Library/LaunchAgents/me.leoank.rc2s2-node.plist
```

### Linux: a systemd user service

`~/.config/systemd/user/rc2s2-node.service`:

```ini
[Unit]
Description=rc2s2 node daemon

[Service]
ExecStart=%h/.nix-profile/bin/rc2s2 node serve --hub http://hub:7700 --token-file %h/.config/rc2s2/node-token
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```console
$ systemctl --user enable --now rc2s2-node
$ loginctl enable-linger $USER     # keep it running when you are logged out
```

### With home-manager

The author's fleet runs the daemon from a home-manager module that writes the launchd
agent or the systemd user service, with an option per flag:
[`rc2s2-node.nix` in neusis](https://github.com/leoank/neusis/blob/main/modules/homeModules/rc2s2-node.nix).
It deliberately leaves `~/.claude/settings.json` alone; the hooks block is still yours
to add.

## 5. The wrapper

For sessions you want to interrupt or kill from the phone, start Claude Code with
`rc2s2 run` instead of `claude`. Everything after `--` goes to `claude`:

```console
$ rc2s2 run
$ rc2s2 run -- --resume
$ rc2s2 run -- --model opus
```

A shell alias makes it the default: `alias claude='rc2s2 run --'`. The wrapper talks to
the daemon over a unix socket under `$XDG_RUNTIME_DIR` (or the temporary directory); if
the daemon is not running, the session still works and the wrapper says it cannot be
interrupted remotely.

Two things the wrapper needs: the working directory must already be trusted in Claude
Code (the trust dialog cannot be answered remotely), and Claude Code's own `CLAUDE*`
environment variables are cleared before it starts, so a session started from inside
another one is a fresh session.

## Checking it

The node appears in the app and in `rc2s2 ctl nodes` as connected. Start a session and it
appears under the node within a couple of seconds. On macOS the daemon logs to the file
named in the plist; on Linux, `journalctl --user -u rc2s2-node`.
