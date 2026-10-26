# Troubleshooting

Where to look first: the daemon's log (the file named in the launchd plist on macOS,
`journalctl --user -u rc2s2-node` on Linux), the hub's (`journalctl -u rc2s2-hub`), and
`rc2s2 ctl nodes` and `rc2s2 ctl sessions` to see what the hub knows.

## A node does not show up, or shows as offline

- The daemon logs `hub link down` with the reason and retries with backoff. A
  connection error means the hub's address or port is wrong, or the machine cannot reach
  the hub over the tailnet (`tailscale ping hub`).
- An `Unauthenticated` error means the token is not in the hub's node-token file, or the
  hub has not been restarted since it was added.
- The node's name is `--name`, or the hostname. Two machines with the same token show up
  as one node.

## A session is listed but its dialogs never reach the phone

The daemon finds sessions through Claude Code's session registry and transcripts even
without hooks, but only the hooks let it hold a dialog.

- Check `~/.claude/settings.json` has the block from `rc2s2 node hooks`, with the same
  address as the daemon's `--hook-addr`.
- Restart sessions that were started before the hooks were added.
- A session whose directory matches an `--ignore-cwd` glob is not tracked at all.

## A dialog showed on the phone but the terminal answered first

That is by design. The daemon lets go of a held dialog when you press a key in a
session under `rc2s2 run`, or after `--hold-secs` (2 minutes) for a plain `claude`
session, and the terminal's dialog takes over.

## A prompt from the phone did not start a turn

- If the session runs in bypass-permissions mode and was not started with `rc2s2 run`,
  Claude Code holds a message from another session until it is approved at the terminal.
  The app and `rc2s2 ctl` say so.
- If Claude was working, a prompt waits until the current turn ends.

## Interrupt and Kill are missing

They exist only for sessions started with `rc2s2 run`. When `rc2s2 run` starts, it says
`no node daemon at …` if it could not reach the daemon; that session cannot be steered.
Check that the daemon is running and that both use the same `--wrapper-socket`, if you
set one.

## No notifications

- The ntfy app must be installed on the phone, and rc2s2 allowed to post notifications
  (Android settings, Apps, rc2s2, Notifications). Open rc2s2 once after installing ntfy
  so it registers.
- The hub logs `push failed` with the push server's answer when it cannot deliver.
- Check that the kind is not switched off with `--no-notify`.

## The rate-limit charts are empty or stale

- Charts from the live endpoint need `--claude-usage` on the node. Without it, the numbers
  are Claude Code's own cached copy, updated only when you run `/usage`.
- `usage endpoint refused, too many requests; backing off` in the daemon's log means
  Anthropic's endpoint asked it to slow down; it resumes on its own.
- `usage poll failed` with the reason covers the rest: no network after the machine
  woke, no Claude login, or a keychain that would not hand over the item.

## The machine sleeps in the middle of a turn

Run the node with `--keep-awake` (macOS). It keeps the machine from idle sleep while any
session is busy. Closing the lid still sleeps it.
