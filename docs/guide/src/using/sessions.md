# Sessions

## The home screen

The home screen lists, from the top:

- **Waiting for you**: every permission dialog and question held on any node, each with
  its buttons, so what needs you is always the first thing you see.
- **Each node**, with whether it is online and its live sessions, newest first. A session
  shows its working directory, whether Claude is working or idle, and a terminal icon
  when it runs under `rc2s2 run`. The chart icon next to a node opens its
  [health](health.md).

Pull down or tap refresh to reload. The app keeps one stream open to the hub and
reconnects on its own with backoff when it breaks; the subtitle says
*Reconnecting to …* while it does.

## A session

Tap a session to open it. The conversation reads loosely like the terminal:

- your prompts as boxed lines, Claude's replies as formatted text, its thinking folded;
- each tool call as one line, such as `Bash  cargo test`, with its output folded to a count
  of lines (or the error's first line) until you tap it;
- an edit shows its diff, a long command its script: the first lines under the call, and
  the whole thing full screen when you tap *N more lines*;
- notices from Claude Code and from rc2s2 (interrupted, killed, a slash command) as small
  captions.

The view follows new output while you are at the bottom; scroll up to read and a
*Jump to latest* button brings you back. Long sessions load their last part first;
*show more* at the top loads earlier history.

The header shows the session's tokens in and out and its estimated cost; *Usage* in the
menu shows the totals per model. Time and tokens also appear under each turn.
Costs are estimates from list prices, counted once per message, subagents included.

### Wrapping long lines

Code, diffs and command output scroll sideways by default. The wrap button in the top
bar (also in the full-screen viewer) wraps them to the screen instead. It is one setting
for the whole app and is remembered.

## Prompting

Type in the prompt bar at the bottom and tap send. What happens depends on the session:

- **Under `rc2s2 run`**: the prompt is typed into the session's terminal, exactly as if
  you had typed it. If Claude is working, Claude Code queues it until the turn ends
  (*Send queues after this turn*).
- **Started with plain `claude`**: the prompt goes through Claude Code's own messaging
  socket and arrives as a message from another session; Claude sees it that way. The
  prompt bar says so before you send.

A session in bypass-permissions mode holds socket messages for approval at its
terminal. The app tells you when that has happened rather than pretending the prompt
landed.

## Interrupting and killing

Only for sessions under `rc2s2 run`:

- **Interrupt** (the stop button in the prompt bar, or the menu): presses Escape in the
  session, like you would at the keyboard, ending the current turn. With text in the
  prompt bar, the stop button interrupts and then sends that text as the next prompt.
- **Kill session** (the menu): ends the Claude Code process after a confirmation. The
  transcript stays; nothing else is saved.

For a session started without the wrapper these are not offered, since there is no
supported way to interrupt it from outside.

## When a session ends

It leaves the home screen's live list, and its screen says *This session has ended*.
With the `ended` notification on, you get a push.
