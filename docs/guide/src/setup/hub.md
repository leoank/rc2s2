# The hub

The hub should run on a machine that is always on and reachable from your nodes and your
phone over Tailscale: a small cloud VM, a home server, a Raspberry Pi. It needs little:
one process, a SQLite file, a few tens of MB of memory.

## What it needs

- `saarthi-hub` from this repository's flake (`packages.<system>.saarthi-hub`). It builds on
  Linux and macOS; the NixOS module below is for Linux.
- A directory on persistent storage for `hub.db`, created on first start.
- Two token files, kept secret:
  - **node tokens**, one line per machine: `<node name> <token>`;
  - **client tokens**, one line per client (the phone, `saarthi ctl`): `<client name> <token>`.

  The hub knows a peer by its token, not its name; the name is what logs and the app show.
  Use long random tokens (`openssl rand -hex 24`), and give every node and client its
  own, so one can be revoked by deleting its line and restarting the hub.
- Optionally, an [ntfy](https://ntfy.sh) topic URL for notifications. The phone's push
  works without it; see [Notifications](notifications.md).

## Running it

```console
$ saarthi-hub --listen 100.101.102.103:7700 --db /var/lib/saarthi-hub/hub.db \
    --node-tokens /run/secrets/saarthi-node-tokens \
    --client-tokens /run/secrets/saarthi-client-tokens \
    --ntfy https://ntfy.example.org/saarthi
```

| Flag | What it does |
|---|---|
| `--listen` | Where nodes and clients connect. Default `127.0.0.1:7700`, reachable only from the same machine. Use the machine's tailnet address, or `0.0.0.0:7700` with the port closed in every firewall except to the tailnet. |
| `--db` | The SQLite database. Back it up like any other SQLite file; it is the hub's whole record. |
| `--node-tokens`, `--client-tokens` | The token files. Read at start; restart the hub after editing them. |
| `--ntfy` | An ntfy topic URL to publish notifications to, besides the phones' own push. |
| `--no-notify <kind>` | Turn one kind of notification off: `permission`, `question`, `turn`, `ended` or `usage`. Repeat for several. |

The hub speaks plain gRPC with no TLS. Tailscale encrypts the traffic between machines;
do not expose the port to the internet.

## On NixOS

A systemd service is all it takes. A minimal module, with the token files from your
secrets manager (agenix, sops-nix):

```nix
{ inputs, pkgs, ... }:
let hub = inputs.saarthi.packages.${pkgs.system}.saarthi-hub; in
{
  users.users.saarthi-hub = { isSystemUser = true; group = "saarthi-hub"; home = "/var/lib/saarthi-hub"; };
  users.groups.saarthi-hub = { };
  systemd.tmpfiles.rules = [ "d /var/lib/saarthi-hub 0750 saarthi-hub saarthi-hub - -" ];
  systemd.services.saarthi-hub = {
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    serviceConfig = {
      User = "saarthi-hub";
      ExecStart = ''
        ${hub}/bin/saarthi-hub --listen 0.0.0.0:7700 --db /var/lib/saarthi-hub/hub.db \
          --node-tokens /run/agenix/saarthiNodeTokens --client-tokens /run/agenix/saarthiClientTokens
      '';
      Restart = "on-failure";
    };
  };
}
```

with `inputs.saarthi.url = "github:leoank/saarthi";` in your flake. The author's own fleet
runs the hub this way; its module, with options for each flag, is
[`rc2s2-hub.nix` in neusis](https://github.com/leoank/neusis/blob/main/modules/nixosModules/rc2s2-hub.nix).

## Checking it

`journalctl -u saarthi-hub` shows `hub serving` at start and one `node registered` line per
node that connects. From any machine with a client token (or with
[`ctl.toml`](../using/ctl.md) set up, just `saarthi ctl nodes`):

```console
$ saarthi ctl --hub http://hub:7700 --token-file ~/.config/saarthi/client-token nodes
```

lists every node the hub knows and whether it is connected.

## Upgrading

Stop the hub, replace the binary, start it. The database is migrated forward
automatically at start. Nodes reconnect on their own within seconds and re-send anything
the hub had not acknowledged; the app reconnects by itself.
