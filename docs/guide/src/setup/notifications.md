# Notifications

The hub sends a notification when:

- a session is waiting on a **permission** dialog;
- Claude asks a **question**;
- a session finishes a **turn**;
- a session has **ended** (exited or was killed);
- one of your Claude rate limits is running out early (**usage**).

It sends each one two ways, and you can use either or both.

## To the app, through UnifiedPush

The app receives push through [UnifiedPush](https://unifiedpush.org), an open standard in
which a *distributor* app on the phone holds one connection to a push server and hands
messages to the apps registered with it. rc2s2 uses the **ntfy** app as the distributor,
so no Google services are needed.

1. Install [ntfy](https://ntfy.sh) on the phone (F-Droid or Play Store). By default it
   uses the public `ntfy.sh` server; to use your own, set the default server in ntfy's
   settings. The hub must be able to reach that server.
2. Open rc2s2 and connect. It registers with ntfy, receives an endpoint URL and gives it
   to the hub.

From then on the hub posts every notification to that endpoint, and rc2s2 shows it as
its own notification. A permission request carries **Allow** and **Deny** buttons that
act without opening the app; tapping any notification opens the session it is about, or
for a usage alert the node's health. If a phone uninstalls the app, the push server
answers 404 or 410 and the hub forgets the endpoint.

## To an ntfy topic

Start the hub with `--ntfy <topic url>`, for example `--ntfy https://ntfy.sh/my-rc2s2-7f3c`
or a topic on your own ntfy server, and subscribe to that topic in the ntfy app or the
ntfy web app on any device. These are plain ntfy messages: a title and a body, no
buttons. Pick an unguessable topic name on a public server, since anyone who knows it
can read it.

## Choosing what you get

Turn off a kind on the hub with `--no-notify`, once per kind:

```console
$ rc2s2-hub … --no-notify turn --no-notify ended
```

The kinds are `permission`, `question`, `turn`, `ended` and `usage`. The setting applies
to both ways of delivery.

## Rate-limit alerts

With `--claude-usage` on a node (see [Rate limits](../using/health.md#rate-limits)), the
hub watches that node's 5-hour, 7-day and 7-day Fable windows and alerts once per window
and reset when one:

| Rule | When | Priority |
|---|---|---|
| reached | the window is at 100% | urgent |
| near | the window passes 80%, and again at 95% | high |
| going fast | a 7-day window is at 20% or more and, at the rate so far, would run out before it resets | high |

For example, three days into a week: "laptop: Claude 7-day limit going fast", "48% used
with 4 days to go; at this pace it runs out in 3 days." Each alert is sent once for a
given window and reset time, even if the hub restarts.
