# goose-roaming

Peer-to-peer transport for goose agents, built on
[iroh](https://iroh.computer) (QUIC, using iroh's public relays for NAT
traversal).

It lets a goose agent accept connections from a remote ACP client (another
goose) that drives it, and lets a client dial a remote agent to hold an
interactive session or delegate a one-shot task, typically without
port-forwarding.

The crate has no dependency on `goose` core, so the iroh dependency stays out of
core. The code that bridges the transport to goose's agent machinery lives in
`goose-cli` behind an optional `roaming` feature; it isn't compiled unless that
feature is enabled.

## Concepts

- **Endpoint identity** — an ed25519 node key that *is* the iroh `EndpointId`.
  It's self-certifying: the QUIC-TLS handshake proves a peer holds the secret
  for the id it claims. Persisted as hex in a `0600` file in goose's config dir.
  Hosts and clients use distinct keys so a node never dials itself.
- **Invite** — a signed, TTL'd token the host mints and shares. It carries the
  host's endpoint id, relay URL(s), a capability scope, and an optional
  client-key allowlist, signed over domain-tagged, length-prefixed canonical
  bytes so it's offline-verifiable and tamper-evident. No relay credentials are
  embedded. Can be marked single-use for pairing.
- **Trust** — how the host authorizes an inbound connection: either *bearer*
  (anyone holding a valid invite) or an explicit *client-key allowlist*.
- **Directory** — an out-of-band record of connections that actually happened
  (inbound and outbound), built purely from observed connections. No gossip.
- **PeerBook** — a user-managed address book of remotes you can connect to by
  nickname; stores the invite as the outbound credential (`0600`).

## Flow

```
host:    bind endpoint ──▶ share(agent) ──▶ mint invite ──▶ (give token to peer)
                                 │
client:  bind endpoint ──▶ decode invite ──▶ dial via relay ──▶ handshake
                                 │
host:    authorize (scope + allowlist) ──▶ hand the bi-stream to ACP serve()
                                 │
client:  run an ACP client over the same bi-stream
```

An iroh bidirectional stream is used as the byte transport for goose's existing
transport-agnostic ACP `serve` / `ByteStreams` seam, so hosting reuses the ACP
server and the client reuses the ACP client.

## CLI

Exposed via `goose roam` (in `goose-cli`, feature `roaming`):

| Command | Purpose |
|---|---|
| `roam share` | Host this machine's agent, print an invite token |
| `roam connect <peer\|token>` | Live interactive session onto the host's agent |
| `roam delegate <peer\|token> "<task>"` | One-shot: send a task, return the response |
| `roam peers save/remove/rename/list` | Address book of remotes |
| `roam connections` | Live/observed connections (no gossip) |
| `roam id` | This machine's host + client endpoint ids |

## Testing across two disconnected machines

Build both with the roaming feature (`cargo build -p goose-cli --features roaming`,
or `just` equivalent) — no shared network, VPN, or port-forwarding needed; the
public n0 relays bridge them. On **machine A** (the host), run `goose roam share`
(optionally `--cwd <dir>` to choose the directory the agent runs in; it defaults
to where `share` started, and the connector's own path is always ignored) and
copy the printed `goose+roam://…` invite token (it embeds A's endpoint id + a
live relay URL; default TTL 1h). Get that token to **machine B** out of band
(paste it in chat, etc.). On **machine B**, either drive A interactively with
`goose roam connect '<token>'` (you'll get a prompt that runs on A's agent — its
tools, files, shell) or hand it a one-shot task with
`goose roam delegate '<token>' "what is 2+2?"` and read the reply. Verify it's
truly A doing the work by asking something machine-specific (e.g. "what's your
hostname and cwd?"). On the host, `goose roam connections` shows who connected;
optionally save the peer on B with `goose roam peers save boxA '<token>'` and
thereafter use the nickname (`goose roam connect boxA`). If session creation
hangs on macOS, prefix with `GOOSE_DISABLE_KEYRING=1` (see PR #10549 for the fix).

## Design decisions & rationale

**`connect` is a thin ACP client UI onto the host's agent — not a provider
wrapper.** The host runs the agent loop (its tools, working directory, shell);
the client just opens an ACP session, sends prompts, and renders
`session/update` notifications. Wrapping the remote as a *provider* for a second
local agent loop would double the loop and defeat the point of sharing an agent.
A fresh agent is created per accepted connection (never shared).

**The host controls the working directory.** ACP's `session/new` carries a cwd,
but the connector's absolute path is meaningless on the host machine — using it
would fail on a path mismatch or, worse, silently run in an unintended directory
that happens to exist. So the host ignores the sent cwd and imposes its own:
the directory `roam share` was started in, or an explicit `--cwd`. The client
sends only a placeholder.

**Scope is Control-only for now.** A connected client can drive the agent and
approve its tool use, which is close to shell access on the host — so only share
with peers you trust. The `Scope` enum keeps `Observe`/`Attach` variants, but
they are not offered in the CLI because the host does not yet enforce them
(that needs a session coordinator). Offering an unenforced scope would be
misleading, so it's left out.

**Delegation guardrails are about cost, not authorization.** The peer is already
trusted, so the concern with agent-to-agent delegation is runaway cost from
loops (A → B → A …). Any loop/cost guardrails belong wherever delegation is
surfaced to the model. The current `delegate` path auto-cancels tool-permission
requests, since there is no human present to answer them.

## Surfacing delegation to the model

The agent can reach other agents with **no new code**: a builtin skill
(`roam-delegate`) documents how to call `goose roam delegate <peer> "<task>"`
via the shell. The skill ships in core but is inert unless the `roaming` CLI
feature is built in. This keeps iroh out of core and adds nothing to maintain;
a richer tool (e.g. a small MCP server) can come later if warranted.

## Roadmap: multi-client attach & remote steering

A key target use case is a phone joining a session already running on a laptop —
watching it stream and injecting steering messages: a daemon streams a timeline
to many clients — one live stream for immediacy, plus an authoritative history
fetch for catch-up.

goose already speaks the right wire protocol: ACP has `session/update`
notifications, `session/prompt`, and an unstable `session/steer` for active
runs. What's missing is a coordinator: today each accepted roaming stream builds
a **fresh** `GooseAcpAgent`, and several of its fields (sessions, active-run map,
`client_cx`) are connection-owned, so two clients can't share one live agent by
construction.

**Chosen shape: a broker in front, not a multi-client agent.** The agent still
speaks ACP to exactly one client; that one client is a
**broker** which re-serves ACP to N roaming peers and applies three routing
rules — fan out `session/update`, funnel `session/prompt`/`steer` (serialized),
and route `session/request_permission` to a single controller. This keeps both
iroh *and* multi-client logic out of goose core. The transport-neutral routing
policy is implemented and unit-tested in [`broker.rs`](src/broker.rs)
(`Router`/`SessionBroker`); the ACP wire adapter is the remaining work.

The minimal path (expert-reviewed), roughly in order:

1. **Session-bound invites.** Separate *resource* from *access* in invite claims
   — `InviteTarget::{Agent, Session(id)}` and `Access::{Observe, Steer, Control}`
   — so a phone invite is capability-bound to exactly one session rather than
   able to reach any session on the host.
2. **A session actor/hub in `goose::acp`** (transport-neutral, iroh-free): one
   live `Agent` per session; serialize prompts/steer; fan out `session/update`
   to every attached connection; route permission/fs requests to a single
   designated controller (the laptop), not every subscriber.
3. **Attach via `session/load`**: a late joiner subscribes, the daemon replays
   the persisted session, buffered live events flush, then switch to live —
   subscribe-before-replay avoids the snapshot/live gap. Reconnect = full replay
   (no cursor protocol yet).
4. **Steer** uses the existing active-run steer request; idle input uses
   `session/prompt`.

Deferred until needed: durable `epoch + sequence` cursors and paged catch-up
(a full timeline), controller handoff, and cross-process session ownership.
Note the roaming endpoint must run **inside the process that owns the live
session** — a separate `goose roam share` process can't attach to an agent
running in the desktop process; for a first demo, either embed sharing in the
owner process or have `roam share` own the session and let both laptop and phone
attach to it.

## What's deferred
- Durable / attachable sessions and a session coordinator (needed before
  observe/attach scopes and "spawn a subagent that outlives the caller").
- Self-hosted relays (public n0 relays are rate-limited).
- Encrypted application-layer envelopes for agent-to-agent messages.

## Prior art

Patterns here were informed by studying a sibling production project that runs
iroh 1.0 for distributed LLM inference: minimal-preset endpoints with custom
relay maps, ALPN-based stream dispatch, signed offline-verifiable bootstrap
tokens over domain-tagged canonical bytes, and a node-key/owner-key separation.
