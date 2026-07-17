# goose-roaming

Peer-to-peer roaming transport for goose agents, built on
[iroh](https://iroh.computer) (QUIC with zero-trust public relays).

It lets a goose agent expose itself over the internet so a remote ACP client
(another goose) can drive it, and lets a client dial remote agents to hold an
interactive session or delegate a one-shot task — all through NAT without
port-forwarding.

This crate is **pure transport**: it has no dependency on `goose` core, and the
heavy iroh dependency never enters the core crate. The one place that bridges
the transport to goose's agent machinery lives in `goose-cli` behind an optional
`roaming` feature, so library consumers pay zero cost when it's disabled.

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

An iroh bidirectional stream is a drop-in transport for goose's existing
transport-agnostic ACP `serve` / `ByteStreams` seam, so hosting reuses the real
ACP server unchanged, and the client reuses the standard ACP client.

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
and copy the printed `goose+roam://…` invite token (it embeds A's endpoint id + a
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
wrapper.** The host runs the real agent loop (its tools, working directory,
shell); the client just opens an ACP session, sends prompts, and renders
`session/update` notifications. Wrapping the remote as a *provider* for a second
local agent loop would double the loop and defeat the point of "share your
agent". A fresh agent is created per accepted connection (never shared).

**Scope is Control-only for now.** A shared agent is handed to a trusted,
authenticated peer, and a control invite is intentionally SSH-equivalent — the
peer driving the agent and approving its tool use is the feature, not a gap. The
`Scope` enum keeps `Observe`/`Attach` variants for the future, but they are not
advertised until they can actually be enforced host-side (which needs a session
coordinator). Advertising an unenforced scope would be false security.

**Delegation cares about loops, not authz.** Because the peer is already
trusted, the risk in agent-to-agent delegation isn't authorization — it's
runaway cost (A → B → A …). Guardrails (bounded turns/deadline, no recursive
roaming into delegated sessions) belong wherever delegation is surfaced to the
model, independent of scopes. Delegated sessions auto-cancel permission requests
since there's no human to answer.

**Secrets never hang the runtime.** Reading the node key or config secrets must
not block; see the keychain-read timeout in goose core's config layer.

## Surfacing delegation to the model

The agent can reach other agents with **no new code**: a builtin skill
(`roam-delegate`) documents how to call `goose roam delegate <peer> "<task>"`
via the shell. The skill ships in core but is inert unless the `roaming` CLI
feature is built in. This keeps iroh out of core and adds nothing to maintain;
a richer tool (e.g. a small MCP server) can come later if warranted.

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
