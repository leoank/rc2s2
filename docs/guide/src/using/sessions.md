# Sessions

## The home screen

The home screen lists, from the top:

- **Waiting for you**: every permission dialog and question held on any node, each with
  its buttons, so what needs you is always the first thing you see.
- **Each node**, with whether it is online and its live sessions, newest first. A session
  shows its working directory, whether Claude is working or idle, a terminal icon when
  it runs under `rc2s2 run`, and a rocket when it was started from the app. The chart
  icon next to a node opens its [health](health.md).

Pull down or tap refresh to reload. The app keeps one stream open to the hub and
reconnects on its own with backoff when it breaks; the subtitle says
*Reconnecting to …* while it does.

## Starting a session

*New session* at the bottom of the home screen starts Claude Code on a node, headless.
It shows only when a connected node allows it: the node must run
with at least one `--spawn-root` (see [A node](../setup/node.md#starting-sessions-from-the-app)).
Pick:

- the **node**, when more than one allows it;
- the **directory**: one of the node's spawn roots, the directory of a live session under
  them, or any path you type below a root. With *Create it if it does not exist* on (the
  default), a path that does not exist yet is made first;
- the **first prompt**, optional: left empty, the session starts idle and you type the
  first prompt in its screen;
- the **model**: Claude Code's default, or Fable, Opus, Sonnet or Haiku.

*Start* opens the new session as soon as its node has started it. It runs with the
node's own Claude login, as `claude -p` with its input and output owned by the node, so:

- prompts, interrupts and kill go straight to the process;
- its permission requests come to the app like any other, and wait as long as a wrapped
  session's (`--wrapped-hold-secs`); one left unanswered is denied, and Claude carries on
  without the tool;
- it cannot ask you questions: AskUserQuestion does not exist in that mode;
- it ends if the node daemon stops, and you cannot attach a terminal to it while it
  runs. Afterwards `claude --resume <session id>` in its directory continues it.

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
- **Started from the app**: the prompt goes to the process's input as your next turn.
- **Started with plain `claude`**: the prompt goes through Claude Code's own messaging
  socket and arrives as a message from another session; Claude sees it that way. The
  prompt bar says so before you send.

A session in bypass-permissions mode holds socket messages for approval at its
terminal. The app tells you when that has happened rather than pretending the prompt
landed.

## Interrupting and killing

**Interrupt** (the stop button in the prompt bar, or the menu) ends the current turn, for
sessions under `rc2s2 run` (it presses Escape, as you would at the keyboard) and sessions
started from the app. With text in the prompt bar, the stop button interrupts and then
sends that text as the next prompt. A session started with plain `claude` cannot be
interrupted from outside, so it has no stop button.

**Kill session** (the menu) ends any session's Claude Code process after a confirmation:
a wrapped one through its wrapper, one started from the app by interrupting it and
closing its input, and one started with plain `claude` by sending it SIGTERM, which
ends that terminal's session too. The transcript stays; nothing else is saved.

## When a session ends

It leaves the home screen's live list, and its screen says *This session has ended*.
With the `ended` notification on, you get a push.
