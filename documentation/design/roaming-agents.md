# Roaming Agents — Design & Research

> Branch: `micn/roaming-agents`
> Status: **Research / early design** (no implementation yet)
> Author: research pass with the `expert` (codex) second opinion + study of the sibling `mesh-llm` project

## 1. What we're trying to build

Give goose agents a **peer-to-peer presence on the internet** using [iroh](https://iroh.computer) (n0's QUIC-based p2p library, v1.0). iroh gives us NAT traversal via public/self-hosted relays and a self-certifying identity (each endpoint *is* an ed25519 key). Deep in goose, this becomes an SDK-like building block we can surface in several ways:

1. **Share a running agent across the wire** — a remote ACP client (goose desktop, another goose, any ACP client) connects to and drives an agent running on your machine, through a relay, without opening ports.
2. **Agent-to-agent collaboration** — from inside the agent loop, an agent can look up its *other* agents (same user's fleet, or trusted peers), send messages, steer them, and collaborate.
3. **Long-lived listeners** — CLI options so `goose acp` / `goose serve` can hold an iroh connection open while the agent loop runs.

The core realization: **goose already speaks ACP over an arbitrary byte stream**, so an iroh bidirectional stream is a drop-in transport. The hard parts are *security, identity, session ownership, and lifecycle* — not the wire.

---

## 2. Why this is a natural fit for goose

goose's ACP layer is transport-agnostic at exactly the right seam:

```rust
// crates/goose/src/acp/server.rs
pub fn serve<R, W>(agent: Arc<GooseAcpAgent>, read: R, write: W)
    -> impl Future<Output = Result<()>>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{ /* runs the JSON-RPC ACP protocol over any byte stream */ }
```

Today `serve` is driven over:
- **stdio** (`goose acp`, `run()` in `server.rs`), and
- **WebSocket/HTTP** via axum with token auth (`goose serve`, `crates/goose/src/acp/transport/mod.rs`).

The **client** side is equally pluggable:

```rust
// crates/goose/src/acp/provider.rs
AcpProvider::connect_with_transport(name, mode, config, transport /* : ConnectTo<Client> */)
```

So goose can be **both** an ACP agent (server) and an ACP client. An iroh bi-stream `(SendStream, RecvStream)` implements `AsyncWrite`/`AsyncRead`, so it plugs directly into both seams. iroh streams are ordered, reliable, flow-controlled QUIC streams — a strictly cleaner substrate than stdio for JSON-RPC.

**Config / key storage** already exists: `Config` with system-keyring-backed secrets and a config dir (`~/.config/goose`), plus `Paths` helpers. Persisting a node key is straightforward.

---

## 3. What we learned from `mesh-llm` (../deez)

`mesh-llm` already runs iroh 1.0 in production for distributed inference. Key patterns worth copying (file citations are into `/Users/micn/Development/deez`):

### Transport
- **iroh 1.0 API surface**: `EndpointId` / `EndpointAddr` / `TransportAddr` (not the old `NodeId`/`NodeAddr`), `presets::Minimal`, `RelayConfig`/`RelayMap`.
- **Endpoint bind** (`mesh/mod.rs:2110`):
  ```rust
  Endpoint::builder(iroh::endpoint::presets::Minimal)
      .secret_key(secret_key)
      .alpns(vec![ALPN_V1.to_vec(), STAGE_ALPN_V2.to_vec()])
      .transport_config(startup_transport_config())
      .relay_mode(relay_mode_for_startup(relay))
      .bind().await
  ```
- **Multiple ALPNs**, branch on `accepting.alpn()` to route subprotocols.
- **Custom relay map** — they never use n0's default relays; they point at self-hosted relays (`mesh/mod.rs:596`) and support **per-relay auth tokens** (`with_auth_token`, `mesh/mod.rs:654`) for zero-trust gating.
- **QUIC tuning matters** for long silent request/response cycles: `keep_alive_interval(10s)`, `max_idle_timeout(300s)`, and **per-path** timers (iroh 1.0 multipath tears down idle paths independently; the last path closing kills a connection mid-work). `mesh/mod.rs:2060`.
- **Stream multiplexing**: every bi-stream starts with a 1-byte type tag, then u32-LE length-prefixed `prost` frames. `protocol/mod.rs:607`.

### Identity & security
- **NodeId == ed25519 public key**, self-certifying at the QUIC-TLS handshake — iroh proves the peer holds that key. This is *the* security primitive. (`node_key.rs`)
- **Two key layers**: transport node key (anonymous, per endpoint) vs. **owner key** (human/tenant identity, ed25519 sign + X25519 encrypt). Owner key **signs a certificate binding owner ↔ node** (`ownership.rs`).
- **Signed bootstrap token** (`requirements.rs:44`): base64url payload with `EndpointAddr`(s) + relays + `expires_at` + issuer pubkey + ed25519 signature over **domain-tagged, length-prefixed canonical bytes** (`b"mesh-llm-bootstrap-token-v1:"`). Offline-verifiable, TTL-revocable.
- **Access control**: `TrustPolicy { Off | PreferOwned | RequireOwned | Allowlist }` + a local `TrustStore` (allowed/revoked keys). Because iroh authenticates the *client's* key at handshake, the host can allowlist **by client public key at accept-time**.
- **Anti-replay admission proof** (`DirectNodeAdmissionProof`, ±30s skew) for the general bearer-token case.
- **Sign-then-encrypt envelopes** (`crypto_box` X25519 + XSalsa20-Poly1305) for confidential owner→owner control messages (`envelope.rs`).

---

## 4. Expert review — corrections that shaped the design

An independent model review (codex, read-only) flagged several issues with the naive first design. These are folded into the design below:

1. **Authorization ≠ authentication.** iroh proves *who* the peer is (`remote_id()`); the app must still decide *what they may do*. Enforce a capability model: `token.audience == our EndpointId`, `token.subject == connection.remote_id()`, plus resource/scope checks. Do this in an **`after_handshake` hook** (runs for both inbound and outbound — check direction).
2. **"Only issuer key may connect" is wrong** — if the host issues the token, that would mean only the host can connect. Bind tokens to the *client's* key, or pin the client key on first redemption.
3. **The ±30s signed proof is largely redundant** — TLS already proves current key possession. Replay protection should come from **binding the capability to `remote_id`** (replay by another key fails naturally) or **one-time redemption** for bearer codes. Don't over-engineer.
4. **Don't call bearer mode "PSK"** — it isn't part of the TLS key exchange. It's a bearer capability.
5. **Never ship relay auth tokens inside an invite.** Share relay URLs only; each endpoint fetches its own relay credential. Also avoid dumping every direct `EndpointAddr` (leaks LAN/private addrs, goes stale) — prefer relay URL + let holepunching upgrade.
6. **Disable 0-RTT** for ACP / any mutating op (iroh warns 0-RTT data is replayable).
7. **The biggest security concern is goose's permission model.** `handle_tool_permission_request` (`server.rs:1907`) sends tool permission choices — including `AllowAlways` — to the connected ACP client and trusts the response. **A full ACP invite is effectively remote shell access.** Peer-agent messaging must get a *much narrower* scope and must NOT inherit permission-approval authority.
8. **Don't share one connection-scoped `GooseAcpAgent` across clients**, and don't claim to share a *live* session while actually spinning independent session runtimes. Introduce a `SessionCoordinator` with a stable `AgentId` (distinct from `EndpointId`), one active runtime per session, and a single-controller lease + observers.
9. **Use iroh's `Router`** rather than a hand-rolled accept loop; supervise tasks; await `Router::shutdown` on teardown. Bound stream counts / receive windows / idle timeouts / task counts.
10. **One node key per running process.** Don't load one global persisted key into multiple concurrent goose processes (ambiguous address/relay ownership). Either a single device-level roaming daemon that hosts multiple agents, or per-process keys certified by a separate device/user identity.
11. **`AcpProvider` client needs hardening before internet use**: it runs transports on a dedicated OS thread with its own current-thread runtime (`provider.rs:177`), `Drop` synchronously joins that thread and sends remote `session/close` without a timeout (`provider.rs:673`, `:1173`) — a stalled peer hangs destruction. It also eagerly creates a session and can't attach to an invited live session (`provider.rs:257`) — needs explicit new-vs-load modes.
12. **Public relays are rate-limited** — n0 recommends dedicated relays for production (like mesh-llm's self-hosted ones).

---

## 5. Proposed architecture

### 5.1 Layering

```text
goose-roaming (new crate / module)  ── the SDK construct
  ├─ RoamingIdentity      persisted ed25519 node key (0600, optional keyring)
  ├─ RoamingEndpoint      owns the iroh Endpoint + Router + relay config
  │    ├─ after_handshake policy hook (key allowlist / revocation / direction)
  │    └─ Router
  │         ├─ ALPN "goose-acp/1"      → capability check → agent factory → serve()
  │         └─ ALPN "goose-roaming/1"  → pairing, presence, directory, mailbox
  ├─ Invite / Capability  signed, TTL'd, audience+subject-bound tokens
  └─ TrustBook            local allow/revoke of client keys + token IDs

SessionCoordinator (local, shared)
  ├─ stable AgentId (≠ EndpointId)
  ├─ one active runtime per session
  ├─ single-controller lease
  └─ observer/subscriber notifications
```

### 5.2 Sharing an agent (server side)

- `goose acp --roam` / `goose serve --roam`: bind the endpoint, start the `Router`, print a **signed invite** (base64url).
- On the `goose-acp/1` ALPN: run the `after_handshake` capability check, then `accept_bi()`, and hand the `(recv, send)` pair to the **existing** `serve<R,W>(agent, recv, send)`. The iroh bi-stream replaces stdio/websocket verbatim.
- **Per-connection agent** from a factory (never a shared one). Enforce client-key allowlist and capability scope at accept.

### 5.3 Connecting as a client

- `goose session --connect <INVITE>`: decode invite → verify signature/TTL/audience → dial `EndpointAddr` over `goose-acp/1` → `open_bi()` → feed the stream to the ACP **client** transport, so the remote agent becomes the local provider.
- Desktop: a small **local loopback ↔ iroh sidecar** in Rust is the cleanest incremental path — Electron keeps its existing WebSocket ACP client unchanged; Rust owns iroh. Treat it as an adapter, not the core protocol.
- Add explicit **new-session vs load-session (attach)** connection modes; harden `AcpProvider` shutdown with cancellation + timeouts first.

### 5.4 Agent-to-agent messaging

- Separate ALPN `goose-roaming/1` with a small message protocol (presence, directory lookup, send-message, optional mailbox). **Do not** reuse the full ACP invite for this — it carries permission-approval authority.
- Expose to the model as narrowly-scoped platform tools, e.g. `platform__roaming_list_agents`, `platform__roaming_send_message`, `platform__roaming_steer`. These make scoped calls, never full ACP tool-permission calls.
- Directory/discovery: start with an explicit trusted-peer book; defer gossip until membership/privacy/revocation semantics are defined.
- Guard against agent-to-agent **loops / fan-out / mutual waits** (runaway cost).

### 5.5 Security defaults (revised)

- **Default**: authenticated ACP transport only. Token is **host-signed, client-key-bound** capability; connection rejected unless `remote_id()` matches an allowlisted / token-bound key.
- **First-time pairing**: short-lived, **single-use** bearer code that **pins the authenticated client `EndpointId` on redemption**.
- **Open reusable bearer capability**: explicit opt-in lower-security mode.
- **No unauthenticated mode** without a conspicuous `--dangerously-*` flag (mirrors existing `--dangerously-unauthenticated` on `goose serve`).
- Token fields: version, domain-separation tag, issuer, audience, subject, resource, scopes, `nbf`, `exp`, token-id. Sign **canonical bytes**, not reserialized JSON. Persist revocations by token-id and client key.
- **Disable 0-RTT.** Share relay **URLs** only, never relay credentials. Don't log invites/addresses/session metadata.

### 5.6 Lifecycle

- Single owner responsible for ordered shutdown: stop accepting → drain/cancel ACP handlers → finish streams → `Router::shutdown` → persist trust state.
- Bound QUIC stream counts, receive windows, idle timeouts, task counts (worst-case memory ∝ streams × window).
- Copy mesh-llm's keep-alive + per-path idle timeout tuning for long silent turns.

---

## 6. MVP scope (recommended)

Ship narrow, defer the ambitious parts until semantics are explicit:

1. `goose-roaming` module: persisted key, `RoamingEndpoint` on `Router`, one ALPN `goose-acp/1`.
2. **Authenticated ACP transport only** — `goose acp --roam` prints an invite; `goose session --connect <INVITE>` dials it.
3. Per-connection agent factory + client-key **allowlist / trusted-peer book** + single-use pairing codes.
4. Explicit new-vs-load session behavior; hardened `AcpProvider` shutdown (cancellation + timeouts).
5. Strict resource limits + ordered shutdown.

**Defer**: gossip discovery, durable messaging/mailbox, simultaneous live-session control, model-facing peer tools (add later by making scoped ACP/roaming calls), sign-then-encrypt envelopes.

---

## 7. Example / theoretical usages

```bash
# Share the agent running here; prints a single-use, client-key-pinned invite
goose acp --roam --pair
#   invite: goose+roam://Aghk...<base64url signed capability>...

# On another machine / another goose: drive that agent over the relay
goose session --connect 'goose+roam://Aghk...'

# Restrict who can connect (allowlist client public keys)
goose serve --roam --allow-key z6Mk...pubkey1 --allow-key z6Mk...pubkey2

# Lower-security shareable bearer link (opt in explicitly)
goose acp --roam --bearer --ttl 1h
```

Theoretical use cases this unlocks:
- **Remote control of a home/work agent** from a laptop or phone-side ACP client without port-forwarding.
- **A fleet of user-owned agents** that discover and delegate to each other (e.g. a "coordinator" agent steering specialized workers on different machines).
- **Pair-agenting**: two people's agents connect to collaborate on a shared task with scoped messaging.
- **Ephemeral share links** for a colleague to observe/assist a running session.
- **CI / build agents** that expose themselves to a central orchestrator over relays.

---

## 8. Open questions / risks to resolve before building

- Exact **capability/scope model** — what does "observe" vs "control" vs "message" mean concretely against ACP methods? (Ties directly into the permission-approval hole in `server.rs:1907`.)
- **Session attach** semantics — can a roaming client join a live local session, or only spawn new ones? `SessionCoordinator` + single-controller lease design.
- **Relay operations** — do we self-host (like mesh-llm's `iroh.link` relays) or lean on n0's rate-limited defaults for a first cut?
- **Desktop integration** — sidecar vs native transport in the Electron app.
- **Key/process model** — device-level daemon vs per-process keys.

---

## Appendix: key file references

**goose (this repo)**
- ACP `serve<R,W>` seam: `crates/goose/src/acp/server.rs` (~line 3133)
- ACP client transport: `crates/goose/src/acp/provider.rs` (`connect_with_transport`, `spawn`, `Drop`)
- HTTP/WS transport + token auth: `crates/goose/src/acp/transport/mod.rs`, `.../auth.rs`
- `goose serve` command: `crates/goose-cli/src/cli.rs` (~line 1392)
- Tool permission handling (the sharp edge): `crates/goose/src/acp/server.rs:1907`
- Config/secrets + keyring: `crates/goose/src/config/base.rs`; paths: `crates/goose/src/config/paths.rs`

**mesh-llm (../deez, reference implementation)**
- Endpoint bind: `crates/mesh-llm-host-runtime/src/mesh/mod.rs:2110`
- QUIC tuning: `...mesh/mod.rs:2060`
- Relay URLs / map / auth: `...mesh/mod.rs:596`, `:654`
- Accept + ALPN dispatch: `...mesh/mod.rs:6310`, `:6386`
- Dial: `...src/protocol/mod.rs:602`
- Framing: `...src/protocol/mod.rs:607`
- Node key: `crates/mesh-llm-identity/src/node_key.rs`
- Owner keys: `.../keys.rs`; ownership cert: `.../ownership.rs`
- Signed bootstrap token: `crates/mesh-llm-host-runtime/src/mesh/requirements.rs:44`
- Sign-then-encrypt envelope: `crates/mesh-llm-identity/src/envelope.rs`
