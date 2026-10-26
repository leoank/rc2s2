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
34c8448e45f342e285d99386e9882fb5  laptop  0.1.0  connected
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

`interrupt` and `kill` work only on sessions under `rc2s2 run`. Each command prints `ok`,
or `ok:` followed by a note from the node, for example when a prompt is held for approval
in a bypass-permissions session, or the reason it could not be done.

Every command and flag is in the [command line reference](../reference/cli.md).
