# ADR-0031: The node keeps the machine from idle-sleeping while a session is busy, when asked

Status: accepted
Date: 2026-10-25

## Context
A prompt sent from the phone reached the node, Claude took it, and the reply never showed on
the phone: rogue's screen had turned off and the laptop went to sleep, taking the node's link
to the hub with it. The node buffers what it could not send and resends on the next
connection, so nothing is lost, but the turn itself stalls until the machine wakes, and the
person on the phone sees a turn that never ends. The owner asked whether the daemon should
keep the machine on, with the state visible and switchable from the app. The alternatives
were leaving sleep to the person (Amphetamine, `caffeinate` by hand, the Energy settings),
keeping the machine awake whenever the node runs, or doing it only while a session is busy.

## Decision
`rc2s2 node serve --keep-awake` holds a `caffeinate -i` child (an idle-sleep assertion, not
a display one: the screen still turns off) while any session the node knows is busy, and
drops it when none is. The state goes out as the health metric `node.keep_awake`, which the
app shows on the node's health screen. The neusis module exposes it as `keepAwake`, off by
default. Switching it from the app is a node command and comes with the other node
commands of phase 2.

## Consequences
A laptop finishing a long turn on battery stays up until the turn ends, then sleeps as
usual; an idle session holds nothing. Lid-close sleep is not prevented. Linux has no
assertion yet (`systemd-inhibit` later); the flag only reports 0 there. The node's hub link
also got keepalive pings (item 15), so a sleep that does happen is noticed and recovered
from within half a minute of waking.
