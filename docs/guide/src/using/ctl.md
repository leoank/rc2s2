# rc2s2 ctl

`rc2s2 ctl` is a command-line client of the hub: everything the app does, from a
terminal or a script. It needs the hub's address and a file holding a client token. Put
them in `~/.config/rc2s2/ctl.toml` (or `$XDG_CONFIG_HOME/rc2s2/ctl.toml`) once:

```toml
hub = "http://hub:7700"
token_file = "~/.config/rc2s2/client-token"
```

The token stays in its own file, so the config holds no secret. `--hub` and
`--token-file`, or the environment variables `RC2S2_HUB` and `RC2S2_TOKEN_FILE`, override
the file for one call: a flag wins over its variable, and both win over the file.

Commands that act on something take the node by name or by id, and the request or
session by its id, as the listings print them.

## Looking

```console
$ rc2s2 ctl nodes
34c8448e45f342e285d99386e9882fb5  laptop  0.1.0  connected  spawn: /Users/me/workspace
$ rc2s2 ctl sessions          # live sessions; --all for every session the hub knows
$ rc2s2 ctl pending           # permission requests and questions waiting
$ rc2s2 ctl watch             # print every change until Ctrl-C
```

## Deciding

```console
$ rc2s2 ctl allow laptop <request id>
$ rc2s2 ctl deny laptop <request id> --message "use the staging bucket instead"
$ rc2s2 ctl answer laptop <request id> --answer "Which database?=Postgres"
```

`answer` takes `--answer "<question>=<answer>"` once per question; for a multi-select
question, repeat it once per label you pick. An answer that is not one of the options is
passed to Claude as free text.

## Steering

```console
$ rc2s2 ctl prompt laptop <session id> "run the tests again"
$ rc2s2 ctl interrupt laptop <session id>
$ rc2s2 ctl interrupt laptop <session id> --then "stop, try the other approach"
$ rc2s2 ctl kill laptop <session id>
```

`interrupt` works on sessions under `rc2s2 run` and sessions started from a client;
`kill` on any session (a plain `claude` gets SIGTERM). Each command prints `ok`, or `ok:`
followed by a note from the node, for example when a prompt is held for approval in a
bypass-permissions session, or the reason it could not be done.

## Starting sessions and switching keep-awake

```console
$ rc2s2 ctl spawn laptop ~/workspace/app "fix the failing test" --model sonnet
0b6c3f1e-6d0e-4c1a-9a51-1f2f6c3d8e77
$ rc2s2 ctl spawn laptop ~/workspace/new-idea --create      # made first, starts idle
$ rc2s2 ctl spawn laptop ~/workspace/app "tidy the imports" --permission-mode bypassPermissions
$ rc2s2 ctl spawn laptop ~/workspace/app "try the new parser" --profile sandbox
$ rc2s2 ctl keep-awake laptop on
```

```console
$ rc2s2 ctl history --node laptop --limit 10    # newest first: id, start, directory, first prompt
$ rc2s2 ctl resume laptop <session id> "and now the docs"
```

`resume` continues a finished session in place, idle without a prompt; see
[Resuming a session](sessions.md#resuming-a-session).

`spawn` starts a headless session in a directory under one of the node's spawn roots
(the `spawn:` column of `nodes`) and prints its id, for `prompt`, `interrupt` and `kill`. Without a prompt the session
starts idle; `--create` makes a missing directory under the root first;
`--permission-mode` takes any mode `claude --permission-mode` does (`acceptEdits`, `plan`,
`bypassPermissions`, ...); `--profile` names one of the node's launch profiles, such as a
sandbox (see [Sandboxed sessions](../setup/node.md#sandboxed-sessions)).
The path is the node's, and `~` there is the node's home. See
[Starting a session](sessions.md#starting-a-session). `keep-awake on` or `off` switches the
node's keep-awake hold, as the switch on its health screen does.

Every command and flag is in the [command line reference](../reference/cli.md).
