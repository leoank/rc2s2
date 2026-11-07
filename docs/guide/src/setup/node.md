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
| `--keep-awake` | Keep the machine from idle-sleeping while any session is busy (macOS, `caffeinate -i`), so a turn finishes when the screen turns off. Lid-close sleep still happens. Off at start by default; the switch on the node's health screen turns it on and off while the daemon runs. |
| `--spawn-root <dir>` | Allow the app to start sessions in this directory or below it. Repeatable; without it, nobody can start sessions on this node. See [Starting sessions from the app](#starting-sessions-from-the-app). |
| `--claude <path>` | The `claude` that started sessions run. Default `claude`, found on your login shell's PATH. |
| `--profiles <file>` | Launch profiles a started session may run under, such as a sandbox. See [Sandboxed sessions](#sandboxed-sessions). |
| `--hook-socket <path>` | Where sandboxed sessions send their hooks. Default `~/.local/state/rc2s2/hook.sock`. |
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

## Starting sessions from the app

To let the app start sessions on this node, give the daemon the directories they may run
in:

```console
$ rc2s2 node serve ... --spawn-root ~/workspace --spawn-root ~/scratch
```

A session can then start in any directory at or below a root, never outside one (the
path is checked after symlinks are resolved). The daemon starts it through your login
shell (`$SHELL -l`), so it gets the PATH and environment a terminal would, even when the
daemon itself runs under launchd or systemd with almost none; `claude` must be on that
PATH, or named with `--claude`. It runs with this machine's own Claude login.

## Sandboxed sessions

On macOS a started session can run confined, under a *launch profile* (ADR-0038). The
session then sees only its working directory, the Nix store and the paths the profile
lists; the rest of your home directory, your SSH keys and other sessions are out of reach.
It has the network, but no inbound connections. It uses its own Claude login, a token you
make once:

```console
$ claude setup-token        # in your own terminal; paste the token into a file
$ chmod 600 ~/.config/rc2s2/claude-oauth-token
```

The profiles live in a TOML file given with `--profiles`:

```toml
[profiles.sandbox]
backend = "seatbelt"
description = "Confined: the working directory, the nix store, git identity and the SSH agent"
state_dir = "~/.local/state/rc2s2/sandbox"     # its Claude config, scratch and home
token_file = "~/.config/rc2s2/claude-oauth-token"
path = ["~/.nix-profile/bin", "/etc/profiles/per-user/me/bin", "/run/current-system/sw/bin"]
readonly = ["/private/etc/profiles", "/private/var/run/current-system", "~/.nix-profile",
            "~/.local/state/nix/profiles", "~/.config/git", "~/.ssh/config", "~/.ssh/known_hosts"]
claude_files = ["CLAUDE.md", "skills", "commands", "agents"]
env = { GIT_CONFIG_GLOBAL = "~/.config/git/config", SSH_AUTH_SOCK = "~/.bitwarden-ssh-agent.sock" }
```

- `state_dir` holds the profile's own Claude config directory, where its sessions'
  transcripts live; History and resume find them there, and a resumed session runs under
  the same profile.
- `path` and `readonly` bring in your tools; the profile directories are links into the
  Nix store, which is always readable.
- `claude_files` links those entries of `~/.claude` into the profile, so a sandboxed
  session follows your `CLAUDE.md` and has your skills.
- `env` adds variables. With `SSH_AUTH_SOCK` naming your agent's socket, the session can
  fetch and push with your keys without seeing them; leave it out to keep it from pushing.
  `nix develop` works through the Nix daemon.

Start a session under it with `rc2s2 ctl spawn ... --profile sandbox`.

## Checking it

The node appears in the app and in `rc2s2 ctl nodes` as connected. Start a session and it
appears under the node within a couple of seconds. On macOS the daemon logs to the file
named in the plist; on Linux, `journalctl --user -u rc2s2-node`.
