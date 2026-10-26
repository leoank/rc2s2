# Permissions and questions

## Permission dialogs

When Claude Code is about to show a permission dialog ("Allow Bash: `rm -rf build`?"),
its hook asks the daemon first and waits. The daemon holds the request, the hub shows it
on the phone and sends a push, and the dialog appears in the terminal at the same time.

From the phone, on the home screen, at the top of the session, or in the notification:

- **Allow**: Claude runs the tool. The terminal shows the dialog was answered by the hook.
- **Deny**: optionally with a reason. Claude sees the reason and can try another way.

From the terminal: press any key, and for a session under `rc2s2 run` the daemon lets go
of the request at once, so the dialog behaves exactly as without rc2s2. For a plain
`claude` session the key press is queued by Claude Code until the hold ends.

If nobody answers, the request falls through to the terminal's dialog after the hold:

| Session | Hold |
|---|---|
| plain `claude` | 2 minutes (`--hold-secs`) |
| under `rc2s2 run` | until a key press, at most 9 minutes (`--wrapped-hold-secs`) |

The request then disappears from the phone and the terminal works as usual. Nothing is
ever decided for you.

## Questions

When Claude asks a question with options (its AskUserQuestion tool), the phone shows the
question with its options as buttons, *Pick any that apply* for a multi-select question,
and a field to **answer in your own words**. Tap **Answer** and Claude continues with your
answer; the terminal never shows the question dialog. Questions are held and fall
through to the terminal exactly like permission dialogs.

## From the command line

`rc2s2 ctl pending` lists what is waiting, and `allow`, `deny` and `answer` decide it;
see [rc2s2 ctl](ctl.md).
