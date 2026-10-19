# rc2s2 architecture

Design for [[PRJ-0002]]. The decisions here come from `docs/architecture-questions.md`;
the facts about Claude Code hooks, Tailscale libraries and mxc were checked against the
official docs on 2026-10-16 and then tested in the phase 0 spikes, [[EXP-0001]] to
[[EXP-0009]], whose findings are folded in below. Phase 1 has started: the cargo workspace
and `crates/proto` exist; nothing else beyond the spikes is built yet.

## What rc2s2 is

A command and control center for agentic coding sessions. A small daemon on every
machine that runs agents reports to one hub. The hub knows every running session, can
pause a session at a decision point and hand the decision to a human on another device,
records every run, and keeps a time series of node health. Clients talk only to the hub.

What it adds over existing remote-driving tools:

- **Many harnesses, many accounts.** Existing tools tie one agent harness to one account.
  rc2s2 treats the harness as an adapter and the account as a credential the hub owns
  and leases to a node for as many sessions as the node wants to run under it.
- **Many networks.** The hub, the nodes and the clients may each sit on different
  tailnets. Every part embeds its own Tailscale identity, so nothing depends on the
  host's network setup.
- **Sessions with declared capabilities.** A session launched by rc2s2 runs in a
  sandbox with an explicit filesystem, network and secret policy, chosen per launch,
  instead of inheriting the whole user account. Interactive sessions you start by hand
  are observed and gated, but not confined.
- **Starts on its own.** Later, inbound messages and scheduled jobs start background
  sessions without a human in the loop.

Scope for now: Claude Code is the only agent, Android is the only client, one person is
the only user.

## Components

```
 phone (android)                      hub machine                        node machines
 ┌─────────────────┐   gRPC/tailnet   ┌──────────────────┐  gRPC/tailnet  ┌─────────────────┐
 │ compose UI      │◄───────────────►│ rc2s2-hub        │◄──────────────►│ rc2s2-node      │
 │ rc2s2-core      │                 │  control service │                │  hook endpoint  │
 │  (rust, uniffi) │   push (ntfy)   │  node service    │                │  pty wrapper    │
 │ tsnet           │◄────────────────│  credential store│                │  transcript     │
 │ ntfy/unifiedpush│                 │  sqlite, ntfy    │                │  health         │
 └─────────────────┘                 │  tsnet x N       │                │  sandbox runtime│
                                     └──────────────────┘                │  tsnet          │
                                                                         └────────┬────────┘
                                                                                  │ hooks, pty
                                                                         ┌────────┴────────┐
                                                                         │ claude          │
                                                                         │ (tty or sandbox)│
                                                                         └─────────────────┘
```

### rc2s2-node

One Rust daemon per machine. It never parses terminal output for meaning; structured
facts come from hooks and the transcript, and the PTY is only a channel for input.

- **Hook endpoint.** Claude Code hooks can be HTTP endpoints, so the daemon listens on
  localhost and a per-user `~/.claude/settings.json` points every event at it. That
  covers every session on the machine, including ones started by hand. Hooks passed with
  `--settings` miss SessionStart, which fires before they load, so the daemon's hooks must
  live in the settings file. Events:
  SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Notification, Stop,
  SubagentStop, SessionEnd, and PermissionRequest. PreToolUse can allow, deny, ask, or
  rewrite the input; PermissionRequest can allow or deny the dialog. Those two are the
  interception points.
- **Transcript tailer.** Hooks carry tool calls and results but not Claude's own text.
  The hook input includes the transcript path, a JSONL file; the daemon tails it to get
  the full conversation for display, and to backfill anything a hook missed. Shapes are in
  [[EXP-0002]]: keep the `uuid`-bearing `user`, `assistant` and `system` lines, ignore
  attachments and state lines, watch the `subagents/` directory next to the file, and
  de-duplicate on `message.id` before summing usage, since one API message is written as one
  line per content block.
- **Session registry.** `~/.claude/sessions/<pid>.json` lists every interactive session with
  pid, session id, cwd, idle or busy status and a messaging socket path ([[EXP-0005]]). The
  daemon reads it to discover sessions that predate it or whose hooks it missed, checking the
  pid since entries outlive their process.
- **PTY wrapper.** There is no supported way to send a prompt or an interrupt to a running
  interactive session from outside; Remote Control is the only mechanism and it is not an
  API. So the daemon ships `rc2s2 run`, which starts `claude` under a PTY it owns and
  mirrors it to your terminal. Through that PTY the hub can type a prompt, send Escape to
  interrupt, or send a signal to kill. A session started without the wrapper is still
  observed and gated through hooks, but cannot be prompted or interrupted.
- **Health collector.** CPU, memory, disk, network, GPU when present, running claude
  processes, and account usage state. One sample every 30 seconds, sent to the hub. GPU on
  macOS comes from IOKit `IOAccelerator` `PerformanceStatistics` (device utilisation, memory
  in use, recovery count) in under 100 µs per sample through `io-kit-sys`; Linux uses NVML or
  the amdgpu sysfs files ([[EXP-0009]]). Rate-limit state ([[EXP-0002]]): spawned sessions
  report it in every `rate_limit_event`; for a leased slot the daemon polls
  `GET /api/oauth/usage` with the slot's token at a 60 s cadence and reports the result in
  its health samples, and the hub does the same for slots it is holding idle
  ([[RUN-20261017-ho7k]]); for a person's own login on a node the only on-disk source is
  `cachedUsageUtilization` in `~/.claude.json`, which nothing but `/usage` writes
  ([[RUN-20261017-trj6]]), so the daemon shows it with its age and runs
  `claude -p "/usage"` there when it is stale, a 4 s call with no model turn.
- **Hub link.** One outbound gRPC bidirectional stream, reconnecting with backoff. The
  daemon buffers events while disconnected so a hub restart loses nothing.
- **Spawner (phase 2).** Starts headless sessions with streamed JSON input and output in
  a chosen directory, owning the permission callback directly.
- **Sandbox runtime (phase 3).** Runs a spawned session inside a containment backend
  under a launch profile. See "Sandboxed sessions".
- **Tailnet identity (phase 3).** Embedded tsnet, so the daemon reaches the hub on the
  hub's tailnet whatever network the host is on.

### rc2s2-hub

One Rust binary, tonic for gRPC, sqlx on SQLite in WAL mode.

- **Node service.** The daemon-facing API: register, stream events, receive commands,
  push health samples, lease credentials.
- **Control service.** The client-facing API. It is a superset of the node service: list
  nodes and sessions across every node, subscribe to a session's event stream, submit
  decisions and prompts, and later spawn sessions. The two services are separate
  protobuf packages from the start since the client sees the hub's view, not a node's.
- **Session registry.** In-memory state of every live session plus its pending requests,
  rebuilt from the event log on start.
- **Event log.** Append-only table of every event from every node. This is the record of
  all runs. Tool inputs and outputs, diffs, usage and cost, and git state at start and
  end are all events.
- **Credential store.** Encrypted at rest. Holds agent account credentials, tailnet
  auth keys, and the tokens nodes and clients present. See "Credentials".
- **Notifier.** Publishes to a self-hosted ntfy server when a session is blocked on a
  human or finishes.
- **Tailnet identities (phase 3).** One embedded tsnet instance per tailnet the hub
  serves.

### rc2s2-proto

Protobuf definitions for both services and the event types. prost and tonic generate
Rust; the Android build generates Kotlin from the same files. This is the one place the
wire format is defined.

### rc2s2-core

The client logic in Rust, exposed to Kotlin through UniFFI: connect to the hub, keep the
node and session list current, stream a session's events, submit commands, and persist
the hub address and token. The Android app is a Compose UI over this core; iOS and the
custom integrations reuse the core later. The embedded tsnet for the phone lives here
too, once it is available for Android.

### Agent adapter

Claude Code is the only implementation, but the daemon talks to it through a trait with
two capability levels:

- **structured**: the harness exposes hooks or an SDK, so events are typed and decisions
  can be injected (Claude Code).
- **terminal**: the harness has no hooks, so the PTY wrapper is all there is. The
  adapter can observe process lifetime, prompt, interrupt, and kill, but cannot gate
  tool calls.

The wrapper from the first level is the whole second level, which is why it exists even
for Claude. The adapter also says how to hand the harness its credentials: for Claude
Code that is a config directory and an environment variable.

## Sandboxed sessions
A session rc2s2 starts runs under a **launch profile**: a named policy the node holds and
the hub catalogs, chosen at spawn time. A profile says:

- **backend**: `host` (no containment, phase 2), a process sandbox (bubblewrap on Linux,
  seatbelt on macOS), a container, or a micro VM.
- **filesystem**: read-only paths, read-write paths, denied paths. The working directory
  and the session state directory are read-write; the rest of the home directory is denied
  unless the profile says otherwise.
- **network**: none, loopback only, allowlisted hosts, or everything, optionally through
  an egress proxy the daemon runs. Per-host allowlists exist only on backends that support
  them; seatbelt offers all or nothing plus a cooperative proxy.
- **secrets**: which leased credentials are written into the state directory, and whether
  the agent sees the real secret or the egress proxy injects it so the sandbox never holds
  it.
- **resources**: CPU, memory, disk, and a wall-clock limit. Only the wall clock is
  enforceable on a process sandbox; the rest needs a container or VM.
- **persistence**: throwaway, or a provisioned container that survives across sessions
  so a workspace keeps its build cache. Not applicable to process sandboxes.

The daemon implements this through a `Containment` trait with the containment backend
behind it. The backend is mxc, Microsoft's sandboxing SDK: a pure Rust crate (`mxc-sdk`)
with one containment model over bubblewrap, LXC and micro VMs on Linux and seatbelt on
macOS, with JSON policies for filesystem, network and UI, and a lifecycle of provision,
start, exec, stop and deprovision for persistent containers. [[EXP-0007]] confirmed it
builds offline under Nix on macOS with no runtime download, and that `claude -p` completes
a turn inside a seatbelt profile with its hooks reaching the daemon and its transcript
readable outside. Its request types have no serde and the generated seatbelt profile cannot
be dumped, so the daemon owns its own policy format and maps it.

What a Claude session needs inside the sandbox, from the minimum policy in [[EXP-0007]]:
read-only `/nix/store` (or the harness closure) and the working directory; read-write the
state directory and a scratch directory, with `CLAUDE_CONFIG_DIR` and `CLAUDE_CODE_TMPDIR`
pointing at them, since claude ignores `TMPDIR` for its own temp files; a `PATH` of the
claude wrapper plus `/usr/bin:/bin`; outbound network; no home directory and no keychain.

The hook endpoint crosses the boundary as a unix socket, not as loopback TCP: on seatbelt,
allowing loopback means allowing all inbound traffic, and it is incompatible with a
proxy-only egress policy. Claude Code's HTTP hooks need a TCP URL, so the settings inside
the sandbox use a command hook, `/usr/bin/curl --unix-socket <state dir>/hooks/hook.sock
--data-binary @- http://hook/hook`, and the daemon listens on that socket. This runs
([[RUN-20261017-syvm]]): with `hostLoopback: deny` the TCP loopback is refused while the
unix socket carries Stop and a held PermissionRequest, with the allow applied, in about six
seconds per turn. One seatbelt subtlety matters for the policy mapping: under
`egress: allow` the profile's unfiltered outbound rule also covers AF_UNIX, so the sandbox
can connect to any host unix socket that is not under a read-only or denied path, which
includes other sessions' `/tmp/cc-socks/*.sock`; the profile therefore denies `/tmp/cc-socks`
and the daemon's own socket directory explicitly, or uses `egress: deny` plus the proxy, under
which only sockets under read-write paths connect.

An interactive session started by hand runs on the host with your own privileges.
rc2s2 observes and gates it but does not confine it; confinement is only for sessions
rc2s2 launches, where it controls the whole environment.

## Credentials
The hub is the only long-term holder of secrets. Three kinds:

- **Agent accounts**: a Claude Code OAuth credential or API key, with a label such as
  `work` or `personal`.
- **Tailnets**: auth keys for the tailnets the hub joins and for enrolling nodes and
  clients onto them.
- **Peers**: the pre-shared token each node and client presents to the hub.

Storage: a `credentials` table of ciphertext, encrypted with a hub master key that lives
in a file outside the database, provided on the hub machine by agenix through neusis.
The threat model is a single trusted operator; the store exists so that secrets never
land in the event log, in a sandbox that should not see them, on a client, or in a
backup of the database in the clear.

Leasing, tested in [[EXP-0008]]: the hub holds credentials; a node borrows one for a
while. When a node is asked to run a session under account `work` and does not already hold
that account, the daemon asks the hub for a lease. The hub returns the secret with a time
limit and the daemon writes it as `.credentials.json` into a **slot directory** on the node,
one directory per account per node. Every session of that account on that node, headless or
interactive, is started with `CLAUDE_CONFIG_DIR` pointing at that directory; with the
variable set claude ignores the host keychain and uses the directory's own store. Any number
of sessions run under one lease at once. A node holds one slot per account it is running,
so several accounts on one node are several directories with several leases, and a session
picks its account by picking the directory. The lease ends when the last session exits and
an idle timeout passes, or when the hub recalls it: the daemon reads the credential back,
since the sessions may have refreshed it, returns it to the hub and wipes the directory. A
node never holds a credential outside a live lease. Clients never receive credentials at
all; the phone sees labels and chooses among them.

The constraint that shapes this ([[RUN-20261017-5jwo]]): a Claude OAuth credential is a
*grant*, and a refresh rotates both tokens and revokes the previous pair at once. Two
*stores* holding one grant are therefore one lease's worth of credential: the first store to
refresh kills the other, including a person's own interactive login if the grant was copied
from their keychain. Many *sessions* on one store are fine, and are what a person's several
terminals do today: the refresh save is a compare-and-swap under a lock file with an
adopt-the-sibling branch, and every running session re-reads the store when it changes, so
one session's refresh is picked up by the rest within seconds (code read in
[[RUN-20261017-5jwo]]; not yet forced in a controlled run with two sessions). Hence the rule:
a grant lives in exactly one store at a time. In the hub an **account** is a set of
**slots**, each slot one grant obtained by one `claude auth login` into a slot-specific
config directory on the hub machine. A slot is leased to at most one node at a time; a
second node wanting the same account gets another slot, or "account busy, add a slot" on the
phone. Separate logins are separate grants and do not interfere, so slots refresh
independently and one person's account can carry as many slots as they are willing to log
in. On macOS a refreshed set lands in the keychain under
`Claude Code-credentials-<sha256(configdir)[:8]>` and the file is deleted, so the read-back
checks that item first; on Linux the file is the store. A sandboxed session needs its slot
directory read-write for the lock file. A dead slot is easy to spot: claude clears it to
empty tokens after `invalid_grant`. `claude auth status` with the slot's config directory
reports `loggedIn` without a turn and is the health probe; `claude auth logout` there
revokes the grant at the server and is how a slot is retired ([[RUN-20261017-tnz7]]). The
hub refreshes an idle slot itself before the access token's eight hours run out, so a lease
always starts with a fresh pair.

Capturing an account: on macOS the live OAuth credential is the login keychain item
"Claude Code-credentials", while `~/.claude/.credentials.json` may be a stale copy; on Linux
the file is the source. Enrollment of an account reads the right one.

An interactive session you start by hand uses whatever credentials are already on the
node. The hub learns the account label from the session's config directory and nothing
more.

Enrollment: a new node or phone needs a tailnet auth key and a hub token before it can
reach the hub, so it cannot fetch them from the hub. The hub mints an **enrollment
bundle**, a one-time blob with the hub's tailnet address, an ephemeral auth key for that
tailnet, and the peer token. On a node it is pasted into `rc2s2 node enroll`; on the
phone it is a QR code. Both keys are revocable from the hub.

## Key flows

### Permission request
1. Claude is about to show a permission dialog; the PermissionRequest hook posts to the
   daemon and blocks. The dialog is drawn in the terminal at the same time.
2. Daemon forwards to the hub; hub records a pending request, streams it to connected
   clients, and publishes an ntfy notification.
3. You tap allow or deny on the phone. Hub forwards to the daemon, daemon answers the
   hook, Claude proceeds. The terminal shows "Allowed by PermissionRequest hook".
4. A keystroke typed at the terminal while the hook is pending is queued and applied only
   after the hook returns. So the daemon releases the hook with an empty response the moment
   the PTY wrapper sees any human keystroke; the queued keystroke then answers the dialog
   within a fraction of a second. No idle timer, no hold-time heuristic. Sessions without the
   wrapper get a fixed hold, long enough for a phone round trip, then release.
5. The per-hook `timeout` is enforced to the second (default 600 s); on cancel the terminal
   dialog simply stays. The daemon sets a large timeout and relies on the release rule.

Measured in [[EXP-0001]]. The `Notification` hook for `permission_prompt` arrives only after
the PermissionRequest hook returns, so the PermissionRequest itself is the push trigger.
PreToolUse is wired the same way for tools that would not show a dialog, so a future policy
engine can gate them too; in phase 1 it only records.

### Question from Claude
AskUserQuestion is a tool, so PreToolUse and then PermissionRequest fire with the questions in
the input. Either hook can answer it ([[EXP-0004]]): return allow with `updatedInput` equal to
the original input plus an `answers` map from question text to the answer: one label, an
array of labels for a multi-select question, or any free text when the person types their
own answer ([[RUN-20261017-nm55]], [[RUN-20261017-nn7f]]). The tool returns at once and no
dialog is shown. rc2s2 uses the PermissionRequest path so the
request is held and released exactly like a permission. The tool does not exist in `-p` mode,
so spawned sessions never block on it.

### Prompt and interrupt
A prompt goes hub to daemon to the session's messaging socket ([[RUN-20261017-ejq2]]):
`~/.claude/sessions/<pid>.json` names `/tmp/cc-socks/<pid>.sock`, and one newline-delimited
JSON line `{"type":"user","message":{"role":"user","content":...}}` from a process of the
same user becomes a user turn, queued behind the current one, with no keystrokes and no
wrapper. This works for sessions started by hand as well, so prompting does not depend on
the PTY wrapper at all. A session in bypass-permissions mode holds such messages for
approval at its terminal, and the daemon reports that state instead of pretending the
prompt landed; `crossSessionInbound: accept` in the session's settings lifts the hold for
sessions rc2s2 starts.

An interrupt has no socket message type, so it goes hub to daemon to PTY and needs the
wrapper. The sequence that works ([[EXP-0003]]): Escape to interrupt, pause, Ctrl-U to
empty the input box (Claude Code refills it with the interrupted prompt), then, if a new
prompt follows, text and Enter. No Stop hook fires for an interrupted turn, so the daemon
marks the turn ended itself. Kill is a signal to the wrapped process. Sessions without the
wrapper reject interrupt with a clear reason that the client shows.

Every `CLAUDE*` environment variable must be stripped before the wrapper starts a session, and
the working directory must already be trusted: the trust dialog cannot be driven through the
PTY. The wrapper never parses terminal output for state; the TUI positions text with cursor
moves and the text is unreliable. Hooks and the transcript are the only sources of truth.

### Spawn (phase 2, sandboxed in phase 3)
1. Client picks a node, a directory from that node's allowed roots, an account, and a
   launch profile. The allowed roots are configured on the node and reported to the hub.
2. Hub checks the grant, leases a slot to the node unless the node already holds one for
   that account, and records the session as starting.
3. Daemon prepares the state directory, writes the credential into the account's slot
   directory if this is a new lease, provisions the sandbox if the profile asks for one,
   and starts `claude -p --input-format stream-json --output-format stream-json` with
   `CLAUDE_CONFIG_DIR` pointing at the slot directory. One process serves many turns, keeps
   context, idles between them and exits when stdin closes ([[EXP-0005]]). Permission requests
   arrive through the same PermissionRequest hook as everywhere else and an allow from the
   hook runs the tool ([[EXP-0001]]); no MCP permission tool is needed. Every turn re-emits
   `init` and a `rate_limit_event`.
4. On exit the daemon wipes the state directory, deprovisions a throwaway sandbox or stops
   a persistent one, and reports the end. When it was the last session on the slot and the
   idle timeout passes, the daemon reads the credential back (it may have refreshed),
   returns it to the hub and wipes the slot directory.

Resume works by session id in both directions, so a terminal session can be continued
headless and a spawned one can be attached at a terminal later. `claude --bg` is the
alternative for sessions the person may want to `attach` to: Claude Code keeps the process
and offers `logs`, `stop` and `rm`, but its output is a terminal stream, so the daemon still
relies on hooks and the transcript.

## Data model

Hub tables, all keyed by hub-assigned ids with the node's own ids stored alongside:

- `nodes`: id, name, tailnets, last seen, capabilities, allowed spawn roots, profiles.
- `accounts`: id, harness, label, credential id.
- `credentials`: id, kind, ciphertext, created, rotated, revoked.
- `grants`: account by node, plus which profiles may use it.
- `profiles`: name, node, backend, policy JSON.
- `sessions`: id, node, account, profile, harness, cwd, started, ended, how it was
  started (wrapper, bare, spawned), transcript path.
- `events`: id, session, sequence, time, kind, payload JSON. Append-only. Payloads are
  redacted of any leased secret before they are written.
- `pending`: open permission requests and questions, with their resolution.
- `health`: node, time, metric, value.

Migrating to Postgres later is a change to the sqlx driver and the migrations; nothing
else should know which database it is.

## Network and auth

The hub, the nodes and the clients may each live on a different tailnet. The end state
is that every part embeds tsnet: the hub holds one identity per tailnet it serves, and
each node and client holds an identity on the hub's tailnet, obtained through its
enrollment bundle, next to whatever tailnet the host already uses for other things.
Nothing relies on the host's tailscaled.

Phase 1 is simpler: everything on one tailnet through the host's tailscaled and the
Tailscale app on the phone, with pre-shared tokens and the hub also recording each
peer's tailnet identity. The hub and daemon get a transport boundary from day one, a
listener and a dialer, so the embedded tsnet lands later without touching the services.

The realistic libraries, checked on 2026-10-16:

- **libtailscale**: full tsnet, multiple handles per process, NAT traversal, certs via
  the local API. Costs a Go toolchain and cgo in the build. [[EXP-0006]] settled the shape:
  a `buildGo127Module` derivation (libtailscale needs Go 1.27) builds the static archive
  with `-buildmode=c-archive`; a hand-written sys crate reads `LIBTAILSCALE_LIB_DIR` and
  links `static=tailscale` plus the CoreFoundation and Security frameworks on Apple
  targets. The published `libtailscale-sys` crate is not used, since its build script runs
  `go build` and downloads modules. Built and run on aarch64-darwin and, through the remote
  builder, x86_64-linux; archive about 27 MB. No Android target; a cross-build with the NDK
  is possible but ours to own.
- **tailscale-rs**, Tailscale's pure Rust tsnet: clean build, but an experimental
  preview with no TLS certs, weaker NAT traversal, and no Android support yet.

Hub and node take libtailscale in phase 3. The phone must embed tsnet as well, decided
2026-10-17: the Tailscale app on the phone may be joined to a different tailnet than the
hub's, and the app cannot hold two. Until phase 3 it uses the Tailscale app on the hub's
tailnet. The Android path is libtailscale cross-built with the NDK inside the client core's
aar; tailscale-rs is the fallback if it gains Android support first. That cross-build is
a phase 3 spike of its own.

## Repository layout

```
code/rc2s2-a-remote-command-and-control-session-server/
  flake.nix              flake-parts; rust toolchain, protobuf, go, android sdk
  Cargo.toml             workspace
  crates/
    proto/               .proto files and generated rust
    hub/
    node/
    core/                uniffi client core
  android/               compose app, consumes core as an aar
  nix/
    hub-module.nix       nixos module
    node-module.nix      home-manager module (launchd and systemd)
  docs/
```

The project flake is a path input of the root flake with nixpkgs following the root, and
it already uses flake-parts; it may add inputs of its own, such as crane or rust-overlay,
followed by `nix flake lock` at the root. Rust, protoc, Go, and the Android SDK all exist
in nixpkgs. The Android SDK needs an unfree licence accepted in the nixpkgs config. The
nix modules live here for now and move to neusis later.

## Phases

### Phase 0: spikes
Done on 2026-10-16, one experiment page each under the project's `experiments/`; findings
are folded into the sections above. [[EXP-0001]] hook round trip, [[EXP-0002]] transcript
shapes and rate limits, [[EXP-0003]] PTY injection, [[EXP-0004]] AskUserQuestion,
[[EXP-0005]] headless multi-turn, resume and background sessions, [[EXP-0006]] libtailscale
under Nix, [[EXP-0007]] mxc seatbelt, [[EXP-0008]] credential leasing, [[EXP-0009]] GPU
metrics on macOS. Throwaway code is under `spikes/`. The open questions it left were closed
on 2026-10-17 with follow-up runs on [[EXP-0002]], [[EXP-0004]], [[EXP-0005]], [[EXP-0007]]
and [[EXP-0008]], the last of which changed the credential model to grants and slots.

### Phase 1: watch and intervene

The daemon, the hub, the proto crate, the client core, the Android app, and the nix
modules, all minimal. Done when, from the phone:

- every node and every live session is listed, with the conversation streaming live;
- a permission dialog can be allowed or denied, and falls through to the terminal when
  ignored;
- a question from Claude can be answered;
- a session under the wrapper can be prompted and interrupted;
- a notification arrives when a session blocks or finishes;
- health shows as a chart per node.

Host tailscaled, one tailnet, pre-shared tokens, SQLite. The adapter trait exists with
one implementation. The credential store exists only for peer tokens.

### Phase 2: launch and own sessions

- Spawn a headless session on a node in a chosen directory, from the phone, under the
  `host` profile.
- Kill any session.
- Credential store holds agent accounts; leases and grants; several accounts per node,
  several sessions per lease, selectable at spawn time.
- Resume a finished session by id.
- Session history browsing in the client: past runs, their events, cost.

### Phase 3: confinement and many networks

- Sandbox runtime through mxc: bubblewrap and seatbelt profiles with filesystem and
  secret policy; persistent workspaces.
- Embedded tsnet in hub and node through libtailscale; tailnet auth keys in the
  credential store; enrollment bundles; hub on its own tailnet.
- libtailscale cross-built for Android with the NDK and embedded in the client core, so
  the phone joins the hub's tailnet regardless of what the Tailscale app is joined to.
- Policy rules on PreToolUse: auto-allow by tool, node, and path.
- A second adapter, terminal-level, for a harness without hooks.
- Health exported to the separate monitoring stack; the hub stops being the long-term
  store for it.
- Postgres.

### Phase 4: autonomous starts and more clients

- Micro VM and container backends; network policy with an egress proxy that injects
  secrets so the sandbox never holds them.
- Triggers: inbound messages and scheduled scrape jobs that start background sessions,
  always under a sandbox profile.
- Embedded tsnet on the phone.
- Web and iOS clients over the same core.
- Integrations into other apps through the control service.

## Open questions
None left from phase 0. New ones go here as they come up.

