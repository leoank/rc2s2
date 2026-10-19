# ADR-0025: The hub pushes to a configurable ntfy topic, a local ntfy in development

Status: accepted
Date: 2026-10-19

## Context
ADR-0008 chose a self-hosted ntfy. The hub machine's NixOS module, which would run it,
comes in step 6, after the app. The alternative for step 5 was the public ntfy.sh, which
would carry session and tool names through a public service.

## Decision
The hub takes `--ntfy <topic url>` and posts one message per held request and per finished
turn to it; without the flag nothing is pushed. In development the topic is a local ntfy
from nixpkgs; on the hub machine it will be the self-hosted service of step 6.

## Consequences
Nothing goes to a public server. The phone needs a way to receive the topic (part 5.5).
