# ADR-0039: Sessions run under named `claude setup-token` tokens the hub holds, not leased slots

Status: accepted
Date: 2026-11-10

## Context
ADR-0018 made an account a set of slots, each one OAuth login, leased to one node at a
time and read back when the lease ends. All of that follows from one fact: a refresh
rotates the token pair and revokes the old one, so two stores of one grant kill each
other ([[RUN-20261017-dssg3d]]). The design costs a login per slot on the hub machine,
lease bookkeeping, reading a refreshed credential back from the node (keychain or file),
the hub refreshing idle slots, and "account busy, add a slot" on the phone. None of it is
built (phase 2 step 3).

`claude setup-token` makes a separate grant whose token lasts a year and never refreshes.
[[EXP-20261107-413q6m]] showed it alone runs headless turns and resumes in an empty config
directory, writes no credential file, and leaves the keychain and the owner's login alone;
ADR-0038 already gives it to sandboxed sessions. With nothing rotating, any number of
stores can hold one token. Slots never added quota either: every slot of one account draws
on the same limits.

The owner decided on 2026-11-10 ([[AGS-20261110-g5mg21]]): the hub stores named tokens;
the person picks one when starting a session; a node has a default token, managed from the
hub and the app; the hub gets the tokens from neusis secrets; until the hub side exists,
the token stays with the node; a token's dates are inferred from when it is added, with a
reminder a month before it expires, and can be set by hand; any node may use any token
for now, a per-token list of nodes later.

## Decision
- **Named tokens from secrets.** The hub reads `--claude-tokens <file>`, lines of
  `<name> <token>` like its node and client token files, provided by agenix through
  neusis and read at start. Adding, renaming, replacing or removing a token is a change in
  neusis and a hub restart. Clients never send or receive a token's value; they see names
  and dates.
- **Dates.** The hub keeps, per name, a fingerprint of the token (a hash, never the value),
  when it was first seen, and when it expires: first seen plus a year unless the person
  has set the expiry from the app or `rc2s2 ctl`. A new fingerprint under the same name is
  a renewed token: its first-seen date restarts and a hand-set expiry is cleared. The
  notifier reminds once, a month before a token expires (beside the alerts of ADR-0032),
  and the app shows each token's expiry.
- **Choosing.** A spawn may name a token. Without one it runs under the node's default
  token; with neither, under the node's own login, which the picker also offers by name
  for host sessions. The default applies to every spawned session, host and sandboxed;
  sessions started by hand in a terminal are never touched. A sandboxed profile with
  neither a token nor a default refuses the spawn. A resume offers the token the session
  last ran under.
- **Defaults** are set per node from the app or `rc2s2 ctl` and kept by the hub.
- **Delivery.** The hub sends the chosen token with the spawn or resume command; the node
  keeps it in memory and passes it as `CLAUDE_CODE_OAUTH_TOKEN` (later, to the egress
  proxy instead, which adds it to requests to the Anthropic API while the sandbox holds a
  dummy; with a fixed token that is one header). Nothing is written to disk, read back or
  returned.
- **Scope.** Any node may use any token. A list of allowed nodes per token comes later;
  until then, cutting off one machine alone means giving it a token of its own.
- **Until the hub side exists**, the token stays with the node: a launch profile's
  `token_file` (ADR-0038), one per profile, as rogue has now.

Settled by the agent, for the owner to revisit:
- The token file's format and its read-at-start behaviour copy the hub's peer token files.
- A renewed token is recognised by its fingerprint changing under the same name.
- "The node's own login" stays a choice in the picker for host sessions, so a node with a
  default can still run a session under it.

Assumptions this rests on, not yet checked against the real thing:
- one token used by several sessions and machines at once keeps working
  ([[EXP-20261110-g5mh7g]]);
- the usage endpoint behind `--claude-usage` accepts such a token, or health needs another
  source ([[EXP-20261110-g5mht5]]);
- one token can be revoked on its own ([[EXP-20261110-g5mh2z]]);
- a sandboxed claude can reach a proxy that adds the token, with no TLS interception
  ([[EXP-20261110-g5mh89]]).

## Alternatives considered
- Leased slots (ADR-0018): needed for refreshing logins; with setup-tokens it is
  machinery for a problem that does not occur.
- An encrypted table on the hub, tokens pasted in from the app or `ctl`: tokens could be
  added without a switch, but the hub would need a master key and an editor for secrets,
  and the secret would pass through the phone; neusis already manages the hub's secrets.
- A token per node and account, the node's grant: finer revocation, but more tokens to
  make and no choice per session.
- Tokens only on the nodes, per profile (today's interim): no hub work, but the app cannot
  choose one, and the proxy would need its own copy.

## Consequences
- ADR-0018 is superseded; phase 2 step 3 becomes the hub reading named tokens, their dates
  and reminders, per-node defaults, and a token chosen at spawn.
- With a node default set, sessions started from the app no longer use the owner's login
  unless picked, so their usage leaves the owner's own rate-limit windows only if the
  token belongs to another Claude account.
- Until the proxy exists, a sandbox can read its token, and a leak is good for up to a
  year, against eight hours for a refreshing login's access token. Revoking it is the
  answer, which [[EXP-20261110-g5mh2z]] has to show is possible; with any node allowed to
  use any token, revoking a shared token stops it everywhere.
- Making a token takes a browser login, once a year per token; a reminder comes a month
  ahead.
- If the usage endpoint refuses the token, a spawned session's rate limits still come
  from its `rate_limit_event`s, but an unused token has no usage figure.
